//! CSV export utilities for benchmark reports.
//!
//! This module keeps CSV configuration, metadata, row construction, document
//! formatting, and file writing split by responsibility while preserving the
//! public CSV export API.

mod aggregate;
mod config;
mod document;
mod low_selectivity_gap;
mod metadata;
mod workload;
mod writer;

pub use aggregate::{
    multi_baseline_aggregate_summary_to_csv, multi_baseline_aggregate_summary_to_csv_with_metadata,
};
pub use config::BenchmarkCsvOutputConfig;
pub use low_selectivity_gap::{
    multi_baseline_low_selectivity_gap_to_csv,
    multi_baseline_low_selectivity_gap_to_csv_with_metadata,
};
pub use metadata::BenchmarkCsvMetadata;
pub use workload::{
    multi_baseline_workload_report_to_csv, multi_baseline_workload_report_to_csv_with_metadata,
};
pub use writer::{
    write_multi_baseline_aggregate_summary_csv,
    write_multi_baseline_aggregate_summary_csv_with_metadata,
    write_multi_baseline_low_selectivity_gap_csv,
    write_multi_baseline_low_selectivity_gap_csv_with_metadata,
    write_multi_baseline_workload_report_csv,
    write_multi_baseline_workload_report_csv_with_metadata,
};
