//! Typed archive timing reports.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::benchmark::math::duration_div;
use crate::benchmark::reports::timing::{
    RepeatedComparisonTimingReport, RepeatedTimingConfig, RepeatedTimingReport, duration_ratio,
    measure_elapsed, measure_repeated, measure_repeated_comparison_interleaved,
};
use crate::build::FSEBuilder;
use crate::data::{FSERecordBatch, FSERecordBatchError, RowId};
use crate::encoding::FSERecordEncoder;
use crate::persistence::{
    FSEArchiveMaintenanceAction, FSEArchiveMaintenancePolicy, FSEArchiveMaintenanceReason,
    FSERecordBatchArchiveError, FSETombstonedTypedQueryIndex,
    FSETombstonedTypedQueryIndexArchiveError, FSETypedQueryIndexAppendDeltaArchiveMaintenanceError,
    FSETypedQueryIndexArchiveCompactionError, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFootprintError, FSETypedQueryIndexArchiveMaintenanceError,
    FSETypedRowTombstoneArchiveError, append_typed_query_index_archive_file,
    compact_typed_query_index_archive_file,
    inspect_typed_query_index_archive_file_maintenance_status,
    inspect_typed_query_index_archive_file_maintenance_status_with_append_batch_archive,
    load_typed_query_index_archive_file, load_typed_query_index_archive_with_tombstones,
    maintain_typed_query_index_archive_file,
    maintain_typed_query_index_archive_file_with_append_batch_archive,
    save_typed_query_index_archive_file, save_typed_record_batch_archive_file,
    save_typed_row_tombstone_archive_file,
    typed_query_index_archive_with_append_delta_and_tombstones_footprint,
    typed_query_index_archive_with_tombstones_footprint,
};
use crate::query::{
    IndexedTypedQueryError, QueryExecutionStats, TypedAppendDeltaQueryView, TypedQueryIndex,
    TypedQueryIndexBuildError, TypedQueryPlan,
};

/// Error returned when typed archive load timing cannot be measured.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedArchiveLoadTimingError {
    /// Saving or loading the typed query index archive failed.
    Archive(FSETypedQueryIndexArchiveError),

    /// Saving or loading the typed row tombstone archive failed.
    Tombstones(FSETypedRowTombstoneArchiveError),

    /// Saving or loading a typed record batch append archive failed.
    AppendArchive(FSERecordBatchArchiveError),

    /// Preparing an append-delta batch for comparison failed.
    AppendDeltaBatch(FSERecordBatchError),

    /// Building a rebuilt typed query index for comparison failed.
    QueryIndexBuild(TypedQueryIndexBuildError),

    /// Compacting the typed query index archive failed.
    ArchiveCompaction(FSETypedQueryIndexArchiveCompactionError),

    /// Loading a typed query index archive with tombstones failed.
    TombstonedArchive(FSETombstonedTypedQueryIndexArchiveError),

    /// Applying typed archive maintenance failed.
    Maintenance(FSETypedQueryIndexArchiveMaintenanceError),

    /// Applying append-delta typed archive maintenance failed.
    AppendDeltaMaintenance(FSETypedQueryIndexAppendDeltaArchiveMaintenanceError),

    /// Reporting typed archive footprint failed.
    ArchiveFootprint(FSETypedQueryIndexArchiveFootprintError),

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
            Self::Tombstones(error) => error.fmt(formatter),
            Self::AppendArchive(error) => error.fmt(formatter),
            Self::AppendDeltaBatch(error) => error.fmt(formatter),
            Self::QueryIndexBuild(error) => error.fmt(formatter),
            Self::ArchiveCompaction(error) => error.fmt(formatter),
            Self::TombstonedArchive(error) => error.fmt(formatter),
            Self::Maintenance(error) => error.fmt(formatter),
            Self::AppendDeltaMaintenance(error) => error.fmt(formatter),
            Self::ArchiveFootprint(error) => error.fmt(formatter),
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
            Self::Tombstones(error) => Some(error),
            Self::AppendArchive(error) => Some(error),
            Self::AppendDeltaBatch(error) => Some(error),
            Self::QueryIndexBuild(error) => Some(error),
            Self::ArchiveCompaction(error) => Some(error),
            Self::TombstonedArchive(error) => Some(error),
            Self::Maintenance(error) => Some(error),
            Self::AppendDeltaMaintenance(error) => Some(error),
            Self::ArchiveFootprint(error) => Some(error),
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

impl From<FSETypedRowTombstoneArchiveError> for TypedArchiveLoadTimingError {
    fn from(error: FSETypedRowTombstoneArchiveError) -> Self {
        Self::Tombstones(error)
    }
}

impl From<FSERecordBatchArchiveError> for TypedArchiveLoadTimingError {
    fn from(error: FSERecordBatchArchiveError) -> Self {
        Self::AppendArchive(error)
    }
}

impl From<FSERecordBatchError> for TypedArchiveLoadTimingError {
    fn from(error: FSERecordBatchError) -> Self {
        Self::AppendDeltaBatch(error)
    }
}

impl From<TypedQueryIndexBuildError> for TypedArchiveLoadTimingError {
    fn from(error: TypedQueryIndexBuildError) -> Self {
        Self::QueryIndexBuild(error)
    }
}

impl From<FSETypedQueryIndexArchiveCompactionError> for TypedArchiveLoadTimingError {
    fn from(error: FSETypedQueryIndexArchiveCompactionError) -> Self {
        Self::ArchiveCompaction(error)
    }
}

impl From<FSETombstonedTypedQueryIndexArchiveError> for TypedArchiveLoadTimingError {
    fn from(error: FSETombstonedTypedQueryIndexArchiveError) -> Self {
        Self::TombstonedArchive(error)
    }
}

impl From<FSETypedQueryIndexArchiveMaintenanceError> for TypedArchiveLoadTimingError {
    fn from(error: FSETypedQueryIndexArchiveMaintenanceError) -> Self {
        Self::Maintenance(error)
    }
}

impl From<FSETypedQueryIndexAppendDeltaArchiveMaintenanceError> for TypedArchiveLoadTimingError {
    fn from(error: FSETypedQueryIndexAppendDeltaArchiveMaintenanceError) -> Self {
        Self::AppendDeltaMaintenance(error)
    }
}

impl From<FSETypedQueryIndexArchiveFootprintError> for TypedArchiveLoadTimingError {
    fn from(error: FSETypedQueryIndexArchiveFootprintError) -> Self {
        Self::ArchiveFootprint(error)
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

    /// Typed query index archive byte length used by the load benchmark.
    pub archive_bytes: u64,

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

/// Timing report for querying an append-delta view against a rebuilt index.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedArchiveAppendDeltaQueryTimingReport {
    /// Number of records in the base typed query index.
    pub base_record_count: usize,

    /// Number of records in the appended batch.
    pub appended_record_count: usize,

    /// Number of records in the equivalent rebuilt typed query index.
    pub rebuilt_record_count: usize,

    /// Number of records matched by append-delta query execution.
    pub append_delta_matched_records: usize,

    /// Number of records matched by rebuilt-index query execution.
    pub rebuilt_matched_records: usize,

    /// Execution statistics for append-delta query execution.
    pub append_delta_stats: QueryExecutionStats,

    /// Execution statistics for rebuilt-index query execution.
    pub rebuilt_stats: QueryExecutionStats,

    /// Repeated timing for rebuilt-index and append-delta query execution.
    pub repeated_timing: RepeatedComparisonTimingReport,

    /// Average rebuilt-index elapsed time divided by average append-delta elapsed time.
    pub rebuilt_to_append_delta_average_ratio: f64,
}

/// Timing report for compacting a tombstoned typed query index archive.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedArchiveCompactionTimingReport {
    /// Number of records in the source archive.
    pub base_record_count: usize,

    /// Number of tombstones used during compaction.
    pub tombstone_count: usize,

    /// Number of source records removed by compaction.
    pub removed_record_count: usize,

    /// Number of source records retained by compaction.
    pub retained_record_count: usize,

    /// Typed query index archive byte length before compaction.
    pub query_archive_bytes_before_compaction: u64,

    /// Typed query index archive byte length after compaction.
    pub query_archive_bytes_after_compaction: u64,

    /// Typed query index archive byte delta after compaction.
    pub query_archive_byte_delta: i128,

    /// Tombstone archive byte length before compaction.
    pub tombstone_archive_bytes_before_compaction: u64,

    /// Tombstone archive byte length after compaction.
    pub tombstone_archive_bytes_after_compaction: u64,

    /// Tombstone archive byte delta after compaction.
    pub tombstone_archive_byte_delta: i128,

    /// Combined query index and tombstone archive byte length before compaction.
    pub logical_archive_bytes_before_compaction: u64,

    /// Combined query index and tombstone archive byte length after compaction.
    pub logical_archive_bytes_after_compaction: u64,

    /// Combined query index and tombstone archive byte delta after compaction.
    pub logical_archive_byte_delta: i128,

    /// Number of records matched by the typed query plan after compaction.
    pub matched_records_after_compaction: usize,

    /// Timing for compaction and archive writes.
    pub compaction_timing: RepeatedTimingReport,
}

/// Timing report for policy-driven typed query index archive maintenance.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedArchiveMaintenanceTimingReport {
    /// Number of records in the source archive.
    pub base_record_count: usize,

    /// Number of records waiting to be appended.
    pub pending_append_record_count: usize,

    /// Number of tombstones waiting to be applied.
    pub tombstone_count: usize,

    /// Maintenance action selected by the policy.
    pub selected_action: FSEArchiveMaintenanceAction,

    /// Reason the maintenance action was selected.
    pub selected_reason: FSEArchiveMaintenanceReason,

    /// Tombstone-to-base-record ratio used by the policy, in basis points.
    pub tombstone_ratio_basis_points: u64,

    /// Whether the inspected maintenance status requires archive writes.
    pub maintenance_status_requires_archive_write: bool,

    /// Number of records in the query archive after maintenance.
    pub resulting_record_count: usize,

    /// Typed query index archive byte length before maintenance.
    pub query_archive_bytes_before_maintenance: u64,

    /// Typed query index archive byte length after maintenance.
    pub query_archive_bytes_after_maintenance: u64,

    /// Typed query index archive byte delta after maintenance.
    pub query_archive_byte_delta: i128,

    /// Tombstone archive byte length before maintenance.
    pub tombstone_archive_bytes_before_maintenance: u64,

    /// Tombstone archive byte length after maintenance.
    pub tombstone_archive_bytes_after_maintenance: u64,

    /// Tombstone archive byte delta after maintenance.
    pub tombstone_archive_byte_delta: i128,

    /// Combined query index and tombstone archive byte length before maintenance.
    pub logical_archive_bytes_before_maintenance: u64,

    /// Combined query index and tombstone archive byte length after maintenance.
    pub logical_archive_bytes_after_maintenance: u64,

    /// Combined query index and tombstone archive byte delta after maintenance.
    pub logical_archive_byte_delta: i128,

    /// Number of records matched by the typed query plan after maintenance.
    pub matched_records_after_maintenance: usize,

    /// Timing for policy evaluation and archive maintenance work.
    pub maintenance_timing: RepeatedTimingReport,
}

/// Timing report for append-delta typed query index archive maintenance.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedArchiveAppendDeltaMaintenanceTimingReport {
    /// Number of records in the source archive.
    pub base_record_count: usize,

    /// Number of records in the append-delta archive before maintenance.
    pub pending_append_record_count: usize,

    /// Number of tombstones waiting to be applied.
    pub tombstone_count: usize,

    /// Maintenance action selected by the policy.
    pub selected_action: FSEArchiveMaintenanceAction,

    /// Reason the maintenance action was selected.
    pub selected_reason: FSEArchiveMaintenanceReason,

    /// Tombstone-to-base-record ratio used by the policy, in basis points.
    pub tombstone_ratio_basis_points: u64,

    /// Whether the inspected maintenance status requires archive writes.
    pub maintenance_status_requires_archive_write: bool,

    /// Number of records in the query archive after maintenance.
    pub resulting_record_count: usize,

    /// Typed query index archive byte length before maintenance.
    pub query_archive_bytes_before_maintenance: u64,

    /// Typed query index archive byte length after maintenance.
    pub query_archive_bytes_after_maintenance: u64,

    /// Typed query index archive byte delta after maintenance.
    pub query_archive_byte_delta: i128,

    /// Append-delta archive byte length before maintenance.
    pub append_archive_bytes_before_maintenance: u64,

    /// Append-delta archive byte length after maintenance.
    pub append_archive_bytes_after_maintenance: u64,

    /// Append-delta archive byte delta after maintenance.
    pub append_archive_byte_delta: i128,

    /// Tombstone archive byte length before maintenance.
    pub tombstone_archive_bytes_before_maintenance: u64,

    /// Tombstone archive byte length after maintenance.
    pub tombstone_archive_bytes_after_maintenance: u64,

    /// Tombstone archive byte delta after maintenance.
    pub tombstone_archive_byte_delta: i128,

    /// Combined archive byte length before maintenance.
    pub logical_archive_bytes_before_maintenance: u64,

    /// Combined archive byte length after maintenance.
    pub logical_archive_bytes_after_maintenance: u64,

    /// Combined archive byte delta after maintenance.
    pub logical_archive_byte_delta: i128,

    /// Number of records matched by the typed query plan after maintenance.
    pub matched_records_after_maintenance: usize,

    /// Timing for persisted append-delta archive maintenance.
    pub maintenance_timing: RepeatedTimingReport,
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

    let archive_bytes = archive_file_len(archive_path)?;
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
        archive_bytes,
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

/// Measures append-delta query execution against an equivalent rebuilt index.
///
/// # Runtime Role
///
/// The function validates that querying the base index plus appended batch
/// returns the same row-id set as querying a rebuilt typed query index. It then
/// reports repeated timing and execution statistics for both paths.
pub fn compare_typed_archive_append_delta_query_execution_repeated(
    query_index: &TypedQueryIndex,
    appended: &FSERecordBatch,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    plan: &TypedQueryPlan,
    timing_config: &RepeatedTimingConfig,
) -> Result<TypedArchiveAppendDeltaQueryTimingReport, TypedArchiveLoadTimingError> {
    let append_delta_view = TypedAppendDeltaQueryView::try_new(query_index, appended)?;
    let append_delta_report = append_delta_view.query_row_ids_with_stats(plan)?;
    let combined_batch = query_index.batch().try_append(appended)?;
    let rebuilt_index = TypedQueryIndex::try_build(combined_batch, encoder, builder)?;
    let rebuilt_report = rebuilt_index.query_row_ids_with_stats(plan)?;

    validate_same_row_id_set(
        "rebuilt typed query index",
        &rebuilt_report.row_ids,
        "append-delta typed query view",
        &append_delta_report.row_ids,
    )?;

    let repeated_timing = measure_repeated_comparison_interleaved(
        timing_config,
        || {
            let report = rebuilt_index
                .query_row_ids_with_stats(plan)
                .expect("rebuilt typed query should match the validated single-run comparison");
            std::hint::black_box(report.row_ids.len());
            std::hint::black_box(report.stats.reconstructed_records);
        },
        || {
            let report = append_delta_view.query_row_ids_with_stats(plan).expect(
                "append-delta typed query should match the validated single-run comparison",
            );
            std::hint::black_box(report.row_ids.len());
            std::hint::black_box(report.stats.reconstructed_records);
        },
    );
    let rebuilt_to_append_delta_average_ratio = duration_ratio(
        repeated_timing.baseline.average_elapsed,
        repeated_timing.fse.average_elapsed,
    );

    Ok(TypedArchiveAppendDeltaQueryTimingReport {
        base_record_count: query_index.batch().len(),
        appended_record_count: appended.len(),
        rebuilt_record_count: rebuilt_index.batch().len(),
        append_delta_matched_records: append_delta_report.row_ids.len(),
        rebuilt_matched_records: rebuilt_report.row_ids.len(),
        append_delta_stats: append_delta_report.stats,
        rebuilt_stats: rebuilt_report.stats,
        repeated_timing,
        rebuilt_to_append_delta_average_ratio,
    })
}

/// Measures typed query index archive compaction timing with repeated timing.
///
/// # Runtime Role
///
/// The function writes a typed query index archive and a tombstone archive,
/// validates the tombstoned result set, compacts the archives, and reports
/// repeated timing for compaction and archive writes.
pub fn compare_typed_archive_compaction_execution_repeated<P, Q>(
    query_archive_path: P,
    tombstone_archive_path: Q,
    query_index: &TypedQueryIndex,
    tombstone_row_ids: &[RowId],
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    plan: &TypedQueryPlan,
    timing_config: &RepeatedTimingConfig,
) -> Result<TypedArchiveCompactionTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_archive_path = query_archive_path.as_ref();
    let tombstone_archive_path = tombstone_archive_path.as_ref();

    save_typed_query_index_archive_file(query_archive_path, query_index)?;
    save_typed_row_tombstone_archive_file(tombstone_archive_path, tombstone_row_ids)?;

    let tombstoned_index =
        load_typed_query_index_archive_with_tombstones(query_archive_path, tombstone_archive_path)?;
    let tombstoned_row_ids = tombstoned_index.query_row_ids(plan)?;
    let footprint_before_compaction = typed_query_index_archive_with_tombstones_footprint(
        query_archive_path,
        tombstone_archive_path,
    )?;
    let query_archive_bytes_before_compaction =
        footprint_before_compaction.query_index_archive_bytes;
    let tombstone_archive_bytes_before_compaction =
        footprint_before_compaction.tombstone_archive_bytes;
    let logical_archive_bytes_before_compaction = footprint_before_compaction.total_archive_bytes;
    let compaction_result = compact_typed_query_index_archive_file(
        query_archive_path,
        tombstone_archive_path,
        encoder,
        builder,
    )?;
    let footprint_after_compaction = typed_query_index_archive_with_tombstones_footprint(
        query_archive_path,
        tombstone_archive_path,
    )?;
    let query_archive_bytes_after_compaction = footprint_after_compaction.query_index_archive_bytes;
    let tombstone_archive_bytes_after_compaction =
        footprint_after_compaction.tombstone_archive_bytes;
    let logical_archive_bytes_after_compaction = footprint_after_compaction.total_archive_bytes;
    let compacted_index =
        load_typed_query_index_archive_with_tombstones(query_archive_path, tombstone_archive_path)?;
    let compacted_row_ids = compacted_index.query_row_ids(plan)?;

    validate_same_row_id_set(
        "tombstoned typed archive",
        &tombstoned_row_ids,
        "compacted typed archive",
        &compacted_row_ids,
    )?;

    let compaction_timing = measure_repeated_archive_compaction(
        query_archive_path,
        tombstone_archive_path,
        query_index,
        tombstone_row_ids,
        encoder,
        builder,
        timing_config,
    )?;

    Ok(TypedArchiveCompactionTimingReport {
        base_record_count: compaction_result.compaction.base_record_count,
        tombstone_count: compaction_result.compaction.tombstone_count,
        removed_record_count: compaction_result.compaction.removed_record_count,
        retained_record_count: compaction_result.compaction.retained_record_count,
        query_archive_byte_delta: byte_delta(
            query_archive_bytes_before_compaction,
            query_archive_bytes_after_compaction,
        ),
        query_archive_bytes_before_compaction,
        query_archive_bytes_after_compaction,
        tombstone_archive_byte_delta: byte_delta(
            tombstone_archive_bytes_before_compaction,
            tombstone_archive_bytes_after_compaction,
        ),
        tombstone_archive_bytes_before_compaction,
        tombstone_archive_bytes_after_compaction,
        logical_archive_byte_delta: byte_delta(
            logical_archive_bytes_before_compaction,
            logical_archive_bytes_after_compaction,
        ),
        logical_archive_bytes_before_compaction,
        logical_archive_bytes_after_compaction,
        matched_records_after_compaction: compacted_row_ids.len(),
        compaction_timing,
    })
}

/// Measures typed query index archive maintenance timing with repeated timing.
///
/// # Runtime Role
///
/// The function writes a typed query index archive and a typed tombstone
/// archive, applies the archive maintenance policy, validates the effective
/// loaded result, and reports repeated timing for the selected maintenance path.
pub fn compare_typed_archive_maintenance_execution_repeated<P, Q>(
    query_archive_path: P,
    tombstone_archive_path: Q,
    query_index: &TypedQueryIndex,
    appended: Option<&FSERecordBatch>,
    tombstone_row_ids: &[RowId],
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
    plan: &TypedQueryPlan,
    timing_config: &RepeatedTimingConfig,
) -> Result<TypedArchiveMaintenanceTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_archive_path = query_archive_path.as_ref();
    let tombstone_archive_path = tombstone_archive_path.as_ref();

    save_typed_query_index_archive_file(query_archive_path, query_index)?;
    save_typed_row_tombstone_archive_file(tombstone_archive_path, tombstone_row_ids)?;

    let maintenance_status = inspect_typed_query_index_archive_file_maintenance_status(
        query_archive_path,
        tombstone_archive_path,
        appended,
        policy,
    )?;
    let footprint_before_maintenance = maintenance_status.footprint;
    let query_archive_bytes_before_maintenance =
        footprint_before_maintenance.query_index_archive_bytes;
    let tombstone_archive_bytes_before_maintenance =
        footprint_before_maintenance.tombstone_archive_bytes;
    let logical_archive_bytes_before_maintenance = footprint_before_maintenance.total_archive_bytes;
    let maintenance_result = maintain_typed_query_index_archive_file(
        query_archive_path,
        tombstone_archive_path,
        appended,
        encoder,
        builder,
        policy,
    )?;
    let footprint_after_maintenance = typed_query_index_archive_with_tombstones_footprint(
        query_archive_path,
        tombstone_archive_path,
    )?;
    let query_archive_bytes_after_maintenance =
        footprint_after_maintenance.query_index_archive_bytes;
    let tombstone_archive_bytes_after_maintenance =
        footprint_after_maintenance.tombstone_archive_bytes;
    let logical_archive_bytes_after_maintenance = footprint_after_maintenance.total_archive_bytes;
    let loaded_effective_index =
        load_typed_query_index_archive_with_tombstones(query_archive_path, tombstone_archive_path)?;
    let surviving_tombstones = surviving_tombstone_row_ids_for_maintenance_action(
        maintenance_result.decision.action,
        tombstone_row_ids,
    );
    let expected_effective_index = FSETombstonedTypedQueryIndex::from_row_ids(
        maintenance_result.query_index.clone(),
        surviving_tombstones.iter().copied(),
    );
    let expected_row_ids = expected_effective_index.query_row_ids(plan)?;
    let loaded_row_ids = loaded_effective_index.query_row_ids(plan)?;

    validate_same_row_id_set(
        "maintained typed archive result",
        &expected_row_ids,
        "loaded maintained typed archive",
        &loaded_row_ids,
    )?;

    let maintenance_timing = measure_repeated_archive_maintenance(
        query_archive_path,
        tombstone_archive_path,
        query_index,
        appended,
        tombstone_row_ids,
        encoder,
        builder,
        policy,
        timing_config,
    )?;

    Ok(TypedArchiveMaintenanceTimingReport {
        base_record_count: maintenance_status.decision.input.base_record_count as usize,
        pending_append_record_count: maintenance_status
            .decision
            .input
            .pending_append_record_count as usize,
        tombstone_count: maintenance_status.decision.input.tombstone_count as usize,
        selected_action: maintenance_status.decision.action,
        selected_reason: maintenance_status.decision.reason,
        tombstone_ratio_basis_points: maintenance_status.decision.tombstone_ratio_basis_points,
        maintenance_status_requires_archive_write: maintenance_status.requires_archive_write(),
        resulting_record_count: maintenance_result.query_index.batch().len(),
        query_archive_byte_delta: byte_delta(
            query_archive_bytes_before_maintenance,
            query_archive_bytes_after_maintenance,
        ),
        query_archive_bytes_before_maintenance,
        query_archive_bytes_after_maintenance,
        tombstone_archive_byte_delta: byte_delta(
            tombstone_archive_bytes_before_maintenance,
            tombstone_archive_bytes_after_maintenance,
        ),
        tombstone_archive_bytes_before_maintenance,
        tombstone_archive_bytes_after_maintenance,
        logical_archive_byte_delta: byte_delta(
            logical_archive_bytes_before_maintenance,
            logical_archive_bytes_after_maintenance,
        ),
        logical_archive_bytes_before_maintenance,
        logical_archive_bytes_after_maintenance,
        matched_records_after_maintenance: loaded_row_ids.len(),
        maintenance_timing,
    })
}

/// Measures persisted append-delta archive maintenance timing with repeated timing.
///
/// # Runtime Role
///
/// The function writes query-index, append-delta, and tombstone archives, applies
/// the archive maintenance policy through the persisted append-delta workflow,
/// validates the loaded result, and reports archive byte movement with timing.
pub fn compare_typed_archive_append_delta_maintenance_execution_repeated<P, Q, R>(
    query_archive_path: P,
    append_archive_path: Q,
    tombstone_archive_path: R,
    query_index: &TypedQueryIndex,
    appended: &FSERecordBatch,
    tombstone_row_ids: &[RowId],
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
    plan: &TypedQueryPlan,
    timing_config: &RepeatedTimingConfig,
) -> Result<TypedArchiveAppendDeltaMaintenanceTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let query_archive_path = query_archive_path.as_ref();
    let append_archive_path = append_archive_path.as_ref();
    let tombstone_archive_path = tombstone_archive_path.as_ref();

    save_typed_query_index_archive_file(query_archive_path, query_index)?;
    save_typed_record_batch_archive_file(append_archive_path, appended)?;
    save_typed_row_tombstone_archive_file(tombstone_archive_path, tombstone_row_ids)?;

    let maintenance_status =
        inspect_typed_query_index_archive_file_maintenance_status_with_append_batch_archive(
            query_archive_path,
            append_archive_path,
            tombstone_archive_path,
            policy,
        )?;
    let footprint_before_maintenance = maintenance_status.footprint;
    let maintenance_result = maintain_typed_query_index_archive_file_with_append_batch_archive(
        query_archive_path,
        append_archive_path,
        tombstone_archive_path,
        encoder,
        builder,
        policy,
    )?;
    let footprint_after_maintenance =
        typed_query_index_archive_with_append_delta_and_tombstones_footprint(
            query_archive_path,
            append_archive_path,
            tombstone_archive_path,
        )?;
    let loaded_effective_index =
        load_typed_query_index_archive_with_tombstones(query_archive_path, tombstone_archive_path)?;
    let surviving_tombstones = surviving_tombstone_row_ids_for_maintenance_action(
        maintenance_result.decision.action,
        tombstone_row_ids,
    );
    let expected_effective_index = FSETombstonedTypedQueryIndex::from_row_ids(
        maintenance_result.query_index.clone(),
        surviving_tombstones.iter().copied(),
    );
    let expected_row_ids = expected_effective_index.query_row_ids(plan)?;
    let loaded_row_ids = loaded_effective_index.query_row_ids(plan)?;

    validate_same_row_id_set(
        "append-delta maintained typed archive result",
        &expected_row_ids,
        "loaded append-delta maintained typed archive",
        &loaded_row_ids,
    )?;

    let maintenance_timing = measure_repeated_archive_append_delta_maintenance(
        query_archive_path,
        append_archive_path,
        tombstone_archive_path,
        query_index,
        appended,
        tombstone_row_ids,
        encoder,
        builder,
        policy,
        timing_config,
    )?;

    Ok(TypedArchiveAppendDeltaMaintenanceTimingReport {
        base_record_count: maintenance_status.decision.input.base_record_count as usize,
        pending_append_record_count: maintenance_status
            .decision
            .input
            .pending_append_record_count as usize,
        tombstone_count: maintenance_status.decision.input.tombstone_count as usize,
        selected_action: maintenance_status.decision.action,
        selected_reason: maintenance_status.decision.reason,
        tombstone_ratio_basis_points: maintenance_status.decision.tombstone_ratio_basis_points,
        maintenance_status_requires_archive_write: maintenance_status.requires_archive_write(),
        resulting_record_count: maintenance_result.query_index.batch().len(),
        query_archive_byte_delta: byte_delta(
            footprint_before_maintenance.query_index_archive_bytes,
            footprint_after_maintenance.query_index_archive_bytes,
        ),
        query_archive_bytes_before_maintenance: footprint_before_maintenance
            .query_index_archive_bytes,
        query_archive_bytes_after_maintenance: footprint_after_maintenance
            .query_index_archive_bytes,
        append_archive_byte_delta: byte_delta(
            footprint_before_maintenance.append_delta_archive_bytes,
            footprint_after_maintenance.append_delta_archive_bytes,
        ),
        append_archive_bytes_before_maintenance: footprint_before_maintenance
            .append_delta_archive_bytes,
        append_archive_bytes_after_maintenance: footprint_after_maintenance
            .append_delta_archive_bytes,
        tombstone_archive_byte_delta: byte_delta(
            footprint_before_maintenance.tombstone_archive_bytes,
            footprint_after_maintenance.tombstone_archive_bytes,
        ),
        tombstone_archive_bytes_before_maintenance: footprint_before_maintenance
            .tombstone_archive_bytes,
        tombstone_archive_bytes_after_maintenance: footprint_after_maintenance
            .tombstone_archive_bytes,
        logical_archive_byte_delta: byte_delta(
            footprint_before_maintenance.total_archive_bytes,
            footprint_after_maintenance.total_archive_bytes,
        ),
        logical_archive_bytes_before_maintenance: footprint_before_maintenance.total_archive_bytes,
        logical_archive_bytes_after_maintenance: footprint_after_maintenance.total_archive_bytes,
        matched_records_after_maintenance: loaded_row_ids.len(),
        maintenance_timing,
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

fn measure_repeated_archive_compaction<P, Q>(
    query_archive_path: P,
    tombstone_archive_path: Q,
    query_index: &TypedQueryIndex,
    tombstone_row_ids: &[RowId],
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    timing_config: &RepeatedTimingConfig,
) -> Result<RepeatedTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_archive_path = query_archive_path.as_ref();
    let tombstone_archive_path = tombstone_archive_path.as_ref();
    let mut total_elapsed = Duration::ZERO;

    for _ in 0..timing_config.iterations {
        save_typed_query_index_archive_file(query_archive_path, query_index)?;
        save_typed_row_tombstone_archive_file(tombstone_archive_path, tombstone_row_ids)?;
        let (compaction_result, elapsed) = measure_elapsed(|| {
            compact_typed_query_index_archive_file(
                query_archive_path,
                tombstone_archive_path,
                encoder,
                builder,
            )
        });
        let compaction_result = compaction_result?;

        std::hint::black_box(compaction_result.compaction.retained_record_count);
        total_elapsed += elapsed;
    }

    Ok(RepeatedTimingReport {
        iterations: timing_config.iterations,
        total_elapsed,
        average_elapsed: duration_div(total_elapsed, timing_config.iterations),
    })
}

fn measure_repeated_archive_maintenance<P, Q>(
    query_archive_path: P,
    tombstone_archive_path: Q,
    query_index: &TypedQueryIndex,
    appended: Option<&FSERecordBatch>,
    tombstone_row_ids: &[RowId],
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
    timing_config: &RepeatedTimingConfig,
) -> Result<RepeatedTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_archive_path = query_archive_path.as_ref();
    let tombstone_archive_path = tombstone_archive_path.as_ref();
    let mut total_elapsed = Duration::ZERO;

    for _ in 0..timing_config.iterations {
        save_typed_query_index_archive_file(query_archive_path, query_index)?;
        save_typed_row_tombstone_archive_file(tombstone_archive_path, tombstone_row_ids)?;
        let (maintenance_result, elapsed) = measure_elapsed(|| {
            maintain_typed_query_index_archive_file(
                query_archive_path,
                tombstone_archive_path,
                appended,
                encoder,
                builder,
                policy,
            )
        });
        let maintenance_result = maintenance_result?;

        std::hint::black_box(maintenance_result.query_index.batch().len());
        std::hint::black_box(maintenance_result.decision.tombstone_ratio_basis_points);
        total_elapsed += elapsed;
    }

    Ok(RepeatedTimingReport {
        iterations: timing_config.iterations,
        total_elapsed,
        average_elapsed: duration_div(total_elapsed, timing_config.iterations),
    })
}

fn measure_repeated_archive_append_delta_maintenance<P, Q, R>(
    query_archive_path: P,
    append_archive_path: Q,
    tombstone_archive_path: R,
    query_index: &TypedQueryIndex,
    appended: &FSERecordBatch,
    tombstone_row_ids: &[RowId],
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
    timing_config: &RepeatedTimingConfig,
) -> Result<RepeatedTimingReport, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let query_archive_path = query_archive_path.as_ref();
    let append_archive_path = append_archive_path.as_ref();
    let tombstone_archive_path = tombstone_archive_path.as_ref();
    let mut total_elapsed = Duration::ZERO;

    for _ in 0..timing_config.iterations {
        save_typed_query_index_archive_file(query_archive_path, query_index)?;
        save_typed_record_batch_archive_file(append_archive_path, appended)?;
        save_typed_row_tombstone_archive_file(tombstone_archive_path, tombstone_row_ids)?;
        let (maintenance_result, elapsed) = measure_elapsed(|| {
            maintain_typed_query_index_archive_file_with_append_batch_archive(
                query_archive_path,
                append_archive_path,
                tombstone_archive_path,
                encoder,
                builder,
                policy,
            )
        });
        let maintenance_result = maintenance_result?;

        std::hint::black_box(maintenance_result.query_index.batch().len());
        std::hint::black_box(maintenance_result.decision.tombstone_ratio_basis_points);
        total_elapsed += elapsed;
    }

    Ok(RepeatedTimingReport {
        iterations: timing_config.iterations,
        total_elapsed,
        average_elapsed: duration_div(total_elapsed, timing_config.iterations),
    })
}

fn surviving_tombstone_row_ids_for_maintenance_action(
    action: FSEArchiveMaintenanceAction,
    tombstone_row_ids: &[RowId],
) -> Vec<RowId> {
    match action {
        FSEArchiveMaintenanceAction::NoMaintenance | FSEArchiveMaintenanceAction::Append => {
            tombstone_row_ids.to_vec()
        }
        FSEArchiveMaintenanceAction::Compact | FSEArchiveMaintenanceAction::Rebuild => Vec::new(),
    }
}

fn archive_file_len(path: &Path) -> Result<u64, TypedArchiveLoadTimingError> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| TypedArchiveLoadTimingError::ArchiveFileMetadata {
            path: path.to_path_buf(),
            kind: error.kind(),
        })
}

fn byte_delta(before: u64, after: u64) -> i128 {
    i128::from(after) - i128::from(before)
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
