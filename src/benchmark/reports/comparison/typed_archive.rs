//! Typed archive load timing reports.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::benchmark::reports::timing::{
    RepeatedTimingConfig, RepeatedTimingReport, duration_ratio, measure_repeated,
};
use crate::data::RowId;
use crate::persistence::{
    FSETypedQueryIndexArchiveError, load_typed_query_index_archive_file,
    save_typed_query_index_archive_file,
};
use crate::query::{IndexedTypedQueryError, TypedQueryIndex, TypedQueryPlan};

/// Error returned when typed archive load timing cannot be measured.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedArchiveLoadTimingError {
    /// Saving or loading the typed query index archive failed.
    Archive(FSETypedQueryIndexArchiveError),

    /// Typed indexed query execution failed.
    Query(IndexedTypedQueryError),

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
