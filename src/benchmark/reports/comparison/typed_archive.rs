//! Typed archive load timing reports.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::benchmark::math::duration_div;
use crate::benchmark::reports::timing::{
    RepeatedTimingConfig, RepeatedTimingReport, duration_ratio, measure_elapsed, measure_repeated,
};
use crate::build::FSEBuilder;
use crate::data::FSERecordBatch;
use crate::data::RowId;
use crate::encoding::FSERecordEncoder;
use crate::persistence::{
    FSETypedQueryIndexArchiveError, append_typed_query_index_archive_file,
    load_typed_query_index_archive_file, save_typed_query_index_archive_file,
};
use crate::query::{IndexedTypedQueryError, TypedQueryIndex, TypedQueryPlan};

/// Error returned when typed archive load timing cannot be measured.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedArchiveLoadTimingError {
    /// Saving or loading the typed query index archive failed.
    Archive(FSETypedQueryIndexArchiveError),

    /// Typed indexed query execution failed.
    Query(IndexedTypedQueryError),

    /// Archive file metadata could not be read.
    ArchiveFileMetadata {
        /// Archive path whose metadata was requested.
        path: PathBuf,

        /// Operating-system error kind.
        kind: io::ErrorKind,
    },

    /// A loaded archive produced a different row-id set.
    ResultMismatch {
        /// Expected result source.
        expected_source: &'static str,

        /// Actual result source.
        actual_source: &'static str,

        /// Sorted expected row identifiers.
        expected: Vec<RowId>,

        /// Sorted actual row identifiers.
        actual: Vec<RowId>,
    },
}

impl fmt::Display for TypedArchiveLoadTimingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(error) => error.fmt(formatter),
            Self::Query(error) => error.fmt(formatter),
            Self::ArchiveFileMetadata { .. } => {
                formatter.write_str("typed archive file metadata could not be read")
            }
            Self::ResultMismatch {
                expected_source,
                actual_source,
                ..
            } => write!(
                formatter,
                "typed archive results from {actual_source} did not match {expected_source}"
            ),
        }
    }
}

impl Error for TypedArchiveLoadTimingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Archive(error) => Some(error),
            Self::Query(error) => Some(error),
            Self::ArchiveFileMetadata { .. } => None,
            Self::ResultMismatch { .. } => None,
        }
    }
}

impl From<FSETypedQueryIndexArchiveError> for TypedArchiveLoadTimingError {
    fn from(error: FSETypedQueryIndexArchiveError) -> Self {
        Self::Archive(error)
    }
}

impl From<IndexedTypedQueryError> for TypedArchiveLoadTimingError {
    fn from(error: IndexedTypedQueryError) -> Self {
        Self::Query(error)
    }
}

/// Timing report for typed indexed query execution across archive load paths.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedArchiveLoadTimingReport {
    /// Number of records matched by the typed query plan.
    pub matched_records: usize,

    /// Timing for querying the existing in-memory typed query index.
    pub in_memory_timing: RepeatedTimingReport,

    /// Timing for querying a typed query index loaded once before measurement.
    pub warm_loaded_timing: RepeatedTimingReport,

    /// Timing for loading a typed query index archive and querying it per iteration.
    pub cold_loaded_timing: RepeatedTimingReport,

    /// Average warm-loaded elapsed time divided by average in-memory elapsed time.
    pub warm_loaded_to_in_memory_ratio: f64,

    /// Average cold-loaded elapsed time divided by average in-memory elapsed time.
    pub cold_loaded_to_in_memory_ratio: f64,

    /// Average cold-loaded elapsed time divided by average warm-loaded elapsed time.
    pub cold_loaded_to_warm_loaded_ratio: f64,
}

/// Timing report for appending typed records and rebuilding an archive.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedArchiveAppendRebuildTimingReport {
    /// Number of records in the source archive.
    pub base_record_count: usize,

    /// Number of records in the append batch.
    pub appended_record_count: usize,

    /// Number of records after the append rebuild.
    pub resulting_record_count: usize,

    /// Archive byte length before append.
    pub archive_bytes_before_append: u64,

    /// Archive byte length after append.
    pub archive_bytes_after_append: u64,

    /// Archive byte growth after append.
    pub archive_byte_growth: u64,

    /// Number of records matched by the typed query plan after append.
    pub matched_records_after_append: usize,

    /// Timing for append, rebuild, and archive write.
    pub append_rebuild_timing: RepeatedTimingReport,
}

/// Measures typed archive load timing with the default repeated timing configuration.
pub fn compare_typed_archive_load_execution<P>(
    archive_path: P,
    query_index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
) -> Result<TypedArchiveLoadTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
{
    compare_typed_archive_load_execution_repeated(
        archive_path,
        query_index,
        plan,
        &RepeatedTimingConfig::default(),
    )
}

/// Measures typed archive load timing with repeated timing.
///
/// # Runtime Role
///
/// The function saves the typed query index to an `.fse` archive, validates that
/// loaded archives preserve the exact row-id set, and reports timing for
/// in-memory, warm-loaded, and cold-loaded query execution.
pub fn compare_typed_archive_load_execution_repeated<P>(
    archive_path: P,
    query_index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    timing_config: &RepeatedTimingConfig,
) -> Result<TypedArchiveLoadTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
{
    let archive_path = archive_path.as_ref();

    save_typed_query_index_archive_file(archive_path, query_index)?;

    let in_memory_row_ids = query_index.query_row_ids(plan)?;
    let warm_loaded_index = load_typed_query_index_archive_file(archive_path)?;
    let warm_loaded_row_ids = warm_loaded_index.query_row_ids(plan)?;

    validate_same_row_id_set(
        "in-memory typed index",
        &in_memory_row_ids,
        "warm-loaded typed archive",
        &warm_loaded_row_ids,
    )?;

    let cold_loaded_index = load_typed_query_index_archive_file(archive_path)?;
    let cold_loaded_row_ids = cold_loaded_index.query_row_ids(plan)?;

    validate_same_row_id_set(
        "in-memory typed index",
        &in_memory_row_ids,
        "cold-loaded typed archive",
        &cold_loaded_row_ids,
    )?;

    let in_memory_timing = measure_repeated(timing_config, || {
        let row_ids = query_index
            .query_row_ids(plan)
            .expect("validated in-memory typed query should execute");
        std::hint::black_box(row_ids.len());
    });
    let warm_loaded_timing = measure_repeated(timing_config, || {
        let row_ids = warm_loaded_index
            .query_row_ids(plan)
            .expect("validated warm-loaded typed query should execute");
        std::hint::black_box(row_ids.len());
    });
    let cold_loaded_timing = measure_repeated(timing_config, || {
        let loaded_index = load_typed_query_index_archive_file(archive_path)
            .expect("validated typed query index archive should load");
        let row_ids = loaded_index
            .query_row_ids(plan)
            .expect("validated cold-loaded typed query should execute");
        std::hint::black_box(row_ids.len());
    });

    Ok(TypedArchiveLoadTimingReport {
        matched_records: in_memory_row_ids.len(),
        warm_loaded_to_in_memory_ratio: duration_ratio(
            warm_loaded_timing.average_elapsed,
            in_memory_timing.average_elapsed,
        ),
        cold_loaded_to_in_memory_ratio: duration_ratio(
            cold_loaded_timing.average_elapsed,
            in_memory_timing.average_elapsed,
        ),
        cold_loaded_to_warm_loaded_ratio: duration_ratio(
            cold_loaded_timing.average_elapsed,
            warm_loaded_timing.average_elapsed,
        ),
        in_memory_timing,
        warm_loaded_timing,
        cold_loaded_timing,
    })
}

/// Measures typed query index archive append rebuild timing with repeated timing.
///
/// # Runtime Role
///
/// The function writes a base typed query index archive, appends a typed record
/// batch through the persisted archive API, validates the loaded result, and
/// reports repeated timing for append, rebuild, and archive write.
pub fn compare_typed_archive_append_rebuild_execution_repeated<P>(
    archive_path: P,
    query_index: &TypedQueryIndex,
    appended: &FSERecordBatch,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    plan: &TypedQueryPlan,
    timing_config: &RepeatedTimingConfig,
) -> Result<TypedArchiveAppendRebuildTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
{
    let archive_path = archive_path.as_ref();

    save_typed_query_index_archive_file(archive_path, query_index)?;
    let archive_bytes_before_append = archive_file_len(archive_path)?;
    let append_result =
        append_typed_query_index_archive_file(archive_path, appended, encoder, builder)?;
    let archive_bytes_after_append = archive_file_len(archive_path)?;
    let loaded_index = load_typed_query_index_archive_file(archive_path)?;
    let appended_row_ids = append_result.query_index.query_row_ids(plan)?;
    let loaded_row_ids = loaded_index.query_row_ids(plan)?;

    validate_same_row_id_set(
        "appended typed query index",
        &appended_row_ids,
        "loaded appended typed archive",
        &loaded_row_ids,
    )?;

    let append_rebuild_timing = measure_repeated_archive_append_rebuild(
        archive_path,
        query_index,
        appended,
        encoder,
        builder,
        timing_config,
    )?;

    Ok(TypedArchiveAppendRebuildTimingReport {
        base_record_count: append_result.append_metadata.base_record_count as usize,
        appended_record_count: append_result.append_metadata.appended_record_count as usize,
        resulting_record_count: append_result.append_metadata.resulting_record_count as usize,
        archive_byte_growth: archive_bytes_after_append.saturating_sub(archive_bytes_before_append),
        archive_bytes_before_append,
        archive_bytes_after_append,
        matched_records_after_append: appended_row_ids.len(),
        append_rebuild_timing,
    })
}

fn measure_repeated_archive_append_rebuild<P>(
    archive_path: P,
    query_index: &TypedQueryIndex,
    appended: &FSERecordBatch,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    timing_config: &RepeatedTimingConfig,
) -> Result<RepeatedTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
{
    let archive_path = archive_path.as_ref();
    let mut total_elapsed = Duration::ZERO;

    for _ in 0..timing_config.iterations {
        save_typed_query_index_archive_file(archive_path, query_index)?;
        let (append_result, elapsed) = measure_elapsed(|| {
            append_typed_query_index_archive_file(archive_path, appended, encoder, builder)
        });
        let append_result = append_result?;

        std::hint::black_box(append_result.query_index.batch().len());
        total_elapsed += elapsed;
    }

    Ok(RepeatedTimingReport {
        iterations: timing_config.iterations,
        total_elapsed,
        average_elapsed: duration_div(total_elapsed, timing_config.iterations),
    })
}

fn archive_file_len(path: &Path) -> Result<u64, TypedArchiveLoadTimingError> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| TypedArchiveLoadTimingError::ArchiveFileMetadata {
            path: path.to_path_buf(),
            kind: error.kind(),
        })
}

fn validate_same_row_id_set(
    expected_source: &'static str,
    expected: &[RowId],
    actual_source: &'static str,
    actual: &[RowId],
) -> Result<(), TypedArchiveLoadTimingError> {
    let mut expected = expected.to_vec();
    let mut actual = actual.to_vec();

    expected.sort_unstable();
    actual.sort_unstable();

    if expected == actual {
        return Ok(());
    }

    Err(TypedArchiveLoadTimingError::ResultMismatch {
        expected_source,
        actual_source,
        expected,
        actual,
    })
}
