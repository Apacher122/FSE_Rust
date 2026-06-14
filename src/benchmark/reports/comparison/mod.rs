//! Comparison utilities for FSE and baseline execution.
//!
//! This module compares exact FSE query execution against exact baseline query
//! paths. Public API wrappers, core comparison execution, and the report type
//! are split so benchmark comparison logic can grow without turning one file
//! into a mixed reporting/execution module.

mod api;
mod execution;
mod report;
mod typed;
mod typed_archive;

pub use api::{
    compare_query_execution, compare_query_execution_repeated,
    compare_query_execution_repeated_with_options, compare_query_execution_with_baseline,
    compare_query_execution_with_baseline_and_options, compare_query_execution_with_options,
};

pub use report::QueryComparisonReport;
pub use typed::{
    TypedQueryComparisonReport, compare_typed_query_execution,
    compare_typed_query_execution_repeated,
};
pub use typed_archive::{
    TypedArchiveAppendRebuildTimingReport, TypedArchiveCompactionTimingReport,
    TypedArchiveLoadTimingError, TypedArchiveLoadTimingReport, TypedArchiveMaintenanceTimingReport,
    compare_typed_archive_append_rebuild_execution_repeated,
    compare_typed_archive_compaction_execution_repeated, compare_typed_archive_load_execution,
    compare_typed_archive_load_execution_repeated,
    compare_typed_archive_maintenance_execution_repeated,
};
