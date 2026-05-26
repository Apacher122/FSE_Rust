//! CSV file-writing helpers.

use std::fs;
use std::io;
use std::path::Path;

use crate::benchmark::reports::multi_summary::MultiBaselineAggregateSummary;
use crate::benchmark::runner::MultiBaselineBenchmarkSuiteReport;

use super::aggregate::{
    multi_baseline_aggregate_summary_to_csv, multi_baseline_aggregate_summary_to_csv_with_metadata,
};
use super::low_selectivity_gap::{
    multi_baseline_low_selectivity_gap_to_csv,
    multi_baseline_low_selectivity_gap_to_csv_with_metadata,
};
use super::metadata::BenchmarkCsvMetadata;
use super::workload::{
    multi_baseline_workload_report_to_csv, multi_baseline_workload_report_to_csv_with_metadata,
};

/// Writes a multi-baseline aggregate summary to a CSV file.
///
/// # Runtime Role
///
/// This is used by tests and callers that only want aggregate rows without run
/// metadata.
pub fn write_multi_baseline_aggregate_summary_csv(
    path: impl AsRef<Path>,
    summary: &MultiBaselineAggregateSummary,
) -> io::Result<()> {
    fs::write(path, multi_baseline_aggregate_summary_to_csv(summary))
}

/// Writes a multi-baseline aggregate summary with run metadata to a CSV file.
///
/// # Runtime Role
///
/// This is used by the benchmark CLI when `--csv-summary` or `--csv` is
/// provided.
pub fn write_multi_baseline_aggregate_summary_csv_with_metadata(
    path: impl AsRef<Path>,
    metadata: &BenchmarkCsvMetadata,
    summary: &MultiBaselineAggregateSummary,
) -> io::Result<()> {
    fs::write(
        path,
        multi_baseline_aggregate_summary_to_csv_with_metadata(metadata, summary),
    )
}

/// Writes a multi-baseline per-workload report to a CSV file.
pub fn write_multi_baseline_workload_report_csv(
    path: impl AsRef<Path>,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> io::Result<()> {
    fs::write(path, multi_baseline_workload_report_to_csv(report))
}

/// Writes a multi-baseline per-workload report with run metadata to a CSV file.
pub fn write_multi_baseline_workload_report_csv_with_metadata(
    path: impl AsRef<Path>,
    metadata: &BenchmarkCsvMetadata,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> io::Result<()> {
    fs::write(
        path,
        multi_baseline_workload_report_to_csv_with_metadata(metadata, report),
    )
}

/// Writes a low-selectivity tree-gap CSV file.
pub fn write_multi_baseline_low_selectivity_gap_csv(
    path: impl AsRef<Path>,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> io::Result<()> {
    fs::write(path, multi_baseline_low_selectivity_gap_to_csv(report))
}

/// Writes a low-selectivity tree-gap CSV file with run metadata.
pub fn write_multi_baseline_low_selectivity_gap_csv_with_metadata(
    path: impl AsRef<Path>,
    metadata: &BenchmarkCsvMetadata,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> io::Result<()> {
    fs::write(
        path,
        multi_baseline_low_selectivity_gap_to_csv_with_metadata(metadata, report),
    )
}
