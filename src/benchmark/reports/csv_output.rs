//! CSV output writing orchestration for benchmark reports.

use std::error::Error;
use std::fmt;
use std::io;

use super::csv::{
    BenchmarkCsvMetadata, BenchmarkCsvOutputConfig,
    write_multi_baseline_aggregate_summary_csv_with_metadata,
    write_multi_baseline_workload_report_csv_with_metadata,
};
use super::multi_summary::MultiBaselineAggregateSummary;
use crate::benchmark::runner::MultiBaselineBenchmarkSuiteReport;

/// CSV export kind used when reporting write failures.
///
/// # Runtime Role
///
/// `BenchmarkCsvOutputKind` identifies which benchmark CSV export path failed
/// without forcing the binary entrypoint to duplicate CSV-specific error text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkCsvOutputKind {
    /// Aggregate summary CSV output.
    Summary,

    /// Per-workload CSV output.
    Workloads,
}

impl BenchmarkCsvOutputKind {
    /// Returns the user-facing label for this CSV output kind.
    pub fn label(&self) -> &'static str {
        match self {
            BenchmarkCsvOutputKind::Summary => "CSV summary",
            BenchmarkCsvOutputKind::Workloads => "workload CSV",
        }
    }
}

/// Report describing which benchmark CSV files were written.
///
/// # Runtime Role
///
/// `BenchmarkCsvWriteReport` lets callers print success messages without
/// knowing the details of how each CSV export is written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkCsvWriteReport {
    /// Path written for the aggregate summary CSV.
    pub summary_path: Option<String>,

    /// Path written for the per-workload CSV.
    pub workloads_path: Option<String>,
}

impl BenchmarkCsvWriteReport {
    /// Returns whether no CSV files were written.
    pub fn is_empty(&self) -> bool {
        self.summary_path.is_none() && self.workloads_path.is_none()
    }

    /// Returns user-facing status lines for completed CSV writes.
    pub fn status_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        if let Some(path) = &self.summary_path {
            lines.push(format!("CSV summary written: {}", path));
        }

        if let Some(path) = &self.workloads_path {
            lines.push(format!("Workload CSV written: {}", path));
        }

        lines
    }
}

/// Error returned when a benchmark CSV file cannot be written.
///
/// # Runtime Role
///
/// `BenchmarkCsvWriteError` preserves the failed path and export kind while
/// wrapping the original I/O error.
#[derive(Debug)]
pub struct BenchmarkCsvWriteError {
    /// CSV output kind that failed.
    pub output_kind: BenchmarkCsvOutputKind,

    /// Output path that failed.
    pub path: String,

    /// Original I/O error.
    pub source: io::Error,
}

impl BenchmarkCsvWriteError {
    fn new(output_kind: BenchmarkCsvOutputKind, path: &str, source: io::Error) -> Self {
        Self {
            output_kind,
            path: path.to_string(),
            source,
        }
    }
}

impl fmt::Display for BenchmarkCsvWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to write {} to `{}`: {}",
            self.output_kind.label(),
            self.path,
            self.source
        )
    }
}

impl Error for BenchmarkCsvWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Writes all configured benchmark CSV outputs.
///
/// # Runtime Role
///
/// This function centralizes CSV write behavior for the benchmark binary. The
/// caller provides the selected CSV paths, run metadata, aggregate summary, and
/// full benchmark report. The function writes only the outputs that were
/// configured.
pub fn write_benchmark_csv_outputs(
    csv_output: &BenchmarkCsvOutputConfig,
    metadata: &BenchmarkCsvMetadata,
    aggregate_summary: &MultiBaselineAggregateSummary,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> Result<BenchmarkCsvWriteReport, BenchmarkCsvWriteError> {
    let mut write_report = BenchmarkCsvWriteReport::default();

    if let Some(path) = &csv_output.summary_path {
        // summary first so the headline csv exists if the second write fails
        write_multi_baseline_aggregate_summary_csv_with_metadata(path, metadata, aggregate_summary)
            .map_err(|source| {
                BenchmarkCsvWriteError::new(BenchmarkCsvOutputKind::Summary, path, source)
            })?;

        write_report.summary_path = Some(path.clone());
    }

    if let Some(path) = &csv_output.workloads_path {
        // workload rows are usually the big file so keep this as the second write
        write_multi_baseline_workload_report_csv_with_metadata(path, metadata, report).map_err(
            |source| BenchmarkCsvWriteError::new(BenchmarkCsvOutputKind::Workloads, path, source),
        )?;

        write_report.workloads_path = Some(path.clone());
    }

    Ok(write_report)
}
