//! CSV export utilities for benchmark reports.

use std::fs;
use std::io;
use std::path::Path;

use super::multi_summary::{BaselineAggregateSummary, MultiBaselineAggregateSummary};
use super::output::BenchmarkRunOverview;
use crate::benchmark::runner::{BaselineBenchmarkSuiteReport, MultiBaselineBenchmarkSuiteReport};

/// Output paths for benchmark CSV exports.
///
/// # Runtime Role
///
/// `BenchmarkCsvOutputConfig` groups CSV output destinations so CLI parsing and
/// benchmark execution do not need to pass each export path as a separate loose
/// field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkCsvOutputConfig {
    /// Optional path for writing the aggregate summary CSV.
    pub summary_path: Option<String>,

    /// Optional path for writing per-workload CSV rows.
    pub workloads_path: Option<String>,
}

impl BenchmarkCsvOutputConfig {
    /// Creates a CSV output configuration from optional paths.
    pub fn new(summary_path: Option<String>, workloads_path: Option<String>) -> Self {
        Self {
            summary_path,
            workloads_path,
        }
    }

    /// Returns whether no CSV output paths were configured.
    pub fn is_empty(&self) -> bool {
        self.summary_path.is_none() && self.workloads_path.is_none()
    }

    /// Returns whether aggregate summary CSV output was configured.
    pub fn has_summary_output(&self) -> bool {
        self.summary_path.is_some()
    }

    /// Returns whether per-workload CSV output was configured.
    pub fn has_workload_output(&self) -> bool {
        self.workloads_path.is_some()
    }

    /// Sets the aggregate summary CSV output path.
    pub fn set_summary_path(&mut self, path: String) {
        // last path wins this matches the cli flag behavior
        self.summary_path = Some(path);
    }

    /// Sets the per-workload CSV output path.
    pub fn set_workloads_path(&mut self, path: String) {
        // same deal here no merge behavior for repeated flags
        self.workloads_path = Some(path);
    }
}

/// Metadata describing the benchmark run that produced a CSV export.
///
/// # Runtime Role
///
/// `BenchmarkCsvMetadata` makes exported benchmark rows self-describing so CSV
/// files can be compared later without relying on terminal output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkCsvMetadata {
    /// Number of records in the selected dataset.
    pub dataset_records: usize,

    /// Number of nodes in the constructed FSE index.
    pub index_nodes: usize,

    /// Number of workload cases in the benchmark run.
    pub workload_count: usize,

    /// Comma-separated baseline names selected for the run.
    pub selected_baselines: String,

    /// Number of repeated timing iterations.
    pub timing_iterations: usize,

    /// Maximum FSE leaf size used during construction.
    pub max_leaf_size: usize,

    /// Maximum FSE build depth used during construction.
    pub max_depth: usize,

    /// Whether all index validation checks passed.
    pub index_valid: bool,

    /// Whether leaf cardinality validation passed.
    pub leaf_cardinality_valid: bool,

    /// Whether hierarchy topology validation passed.
    pub hierarchy_topology_valid: bool,

    /// Whether parent-child bounds validation passed.
    pub parent_child_bounds_valid: bool,
}

impl BenchmarkCsvMetadata {
    /// Builds CSV metadata from the benchmark overview used for terminal output.
    pub fn from_overview(overview: &BenchmarkRunOverview) -> Self {
        Self {
            dataset_records: overview.dataset_records,
            index_nodes: overview.index_nodes,
            workload_count: overview.workloads,
            selected_baselines: overview.baselines.clone(),
            timing_iterations: overview.timing_iterations,
            max_leaf_size: overview.max_leaf_size,
            max_depth: overview.max_depth,
            index_valid: overview.validation.is_valid(),
            leaf_cardinality_valid: overview.validation.leaf_cardinality_valid,
            hierarchy_topology_valid: overview.validation.hierarchy_topology_valid,
            parent_child_bounds_valid: overview.validation.parent_child_bounds_valid,
        }
    }
}

/// Converts a multi-baseline aggregate summary into CSV text.
///
/// # Runtime Role
///
/// This function provides a simple export format for benchmark aggregate data so
/// results can be inspected in spreadsheets, copied into notes, or used by later
/// plotting scripts.
pub fn multi_baseline_aggregate_summary_to_csv(summary: &MultiBaselineAggregateSummary) -> String {
    csv_document(
        aggregate_header_fields(),
        summary
            .baseline_summaries
            .iter()
            .map(aggregate_value_fields),
    )
}

/// Converts a multi-baseline aggregate summary into CSV text with run metadata.
pub fn multi_baseline_aggregate_summary_to_csv_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    summary: &MultiBaselineAggregateSummary,
) -> String {
    csv_document(
        header_fields_with_metadata(aggregate_header_fields()),
        summary
            .baseline_summaries
            .iter()
            .map(|baseline| value_fields_with_metadata(metadata, aggregate_value_fields(baseline))),
    )
}

/// Converts a multi-baseline benchmark report into per-workload CSV text.
pub fn multi_baseline_workload_report_to_csv(report: &MultiBaselineBenchmarkSuiteReport) -> String {
    csv_document(workload_header_fields(), workload_value_rows(report))
}

/// Converts a multi-baseline benchmark report into per-workload CSV text with run metadata.
pub fn multi_baseline_workload_report_to_csv_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> String {
    csv_document(
        header_fields_with_metadata(workload_header_fields()),
        workload_value_rows(report)
            .into_iter()
            .map(|fields| value_fields_with_metadata(metadata, fields)),
    )
}

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

fn workload_value_rows(report: &MultiBaselineBenchmarkSuiteReport) -> Vec<Vec<String>> {
    let mut rows = Vec::new();

    for baseline_report in &report.baseline_reports {
        append_workload_value_rows(&mut rows, baseline_report);
    }

    rows
}

fn append_workload_value_rows(
    rows: &mut Vec<Vec<String>>,
    baseline_report: &BaselineBenchmarkSuiteReport,
) {
    for (summary, pruning_report) in baseline_report
        .report
        .comparisons
        .iter()
        .zip(&baseline_report.report.pruning_reports)
    {
        let comparison = &summary.comparison;
        let pruning = &pruning_report.pruning;

        rows.push(vec![
            baseline_report.baseline_name.clone(),
            comparison.labels.baseline_label.clone(),
            comparison.labels.comparison_label.clone(),
            summary.workload_name.clone(),
            comparison.baseline_stats.evaluated_records.to_string(),
            comparison.baseline_stats.matched_records.to_string(),
            comparison.fse_stats.visited_nodes.to_string(),
            comparison.fse_stats.retained_leaves.to_string(),
            comparison.fse_stats.reconstructed_records.to_string(),
            comparison.fse_stats.matched_records.to_string(),
            comparison.avoided_reconstructions.to_string(),
            format_ratio(comparison.reconstruction_avoidance_ratio as f64),
            format_ratio(comparison.candidate_ratio as f64),
            format_ratio(comparison.retained_leaf_ratio as f64),
            format_ratio(pruning.record_pruning_efficiency as f64),
            format_ratio(pruning.leaf_pruning_efficiency as f64),
            format_ratio(comparison.single_run_timing_ratio),
            format_ratio(comparison.average_timing_ratio),
            comparison.timing.baseline_elapsed.as_nanos().to_string(),
            comparison.timing.fse_elapsed.as_nanos().to_string(),
            comparison
                .repeated_timing
                .baseline
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison
                .repeated_timing
                .fse
                .average_elapsed
                .as_nanos()
                .to_string(),
        ]);
    }
}

fn metadata_header_fields() -> Vec<&'static str> {
    vec![
        "dataset_records",
        "index_nodes",
        "run_workload_count",
        "selected_baselines",
        "timing_iterations",
        "max_leaf_size",
        "max_depth",
        "index_valid",
        "leaf_cardinality_valid",
        "hierarchy_topology_valid",
        "parent_child_bounds_valid",
    ]
}

fn metadata_value_fields(metadata: &BenchmarkCsvMetadata) -> Vec<String> {
    vec![
        metadata.dataset_records.to_string(),
        metadata.index_nodes.to_string(),
        metadata.workload_count.to_string(),
        metadata.selected_baselines.clone(),
        metadata.timing_iterations.to_string(),
        metadata.max_leaf_size.to_string(),
        metadata.max_depth.to_string(),
        metadata.index_valid.to_string(),
        metadata.leaf_cardinality_valid.to_string(),
        metadata.hierarchy_topology_valid.to_string(),
        metadata.parent_child_bounds_valid.to_string(),
    ]
}

fn aggregate_header_fields() -> Vec<&'static str> {
    vec![
        "baseline_name",
        "baseline_label",
        "comparison_label",
        "workload_count",
        "total_baseline_evaluated_records",
        "total_fse_reconstructed_records",
        "weighted_reconstruction_avoidance_ratio",
        "weighted_candidate_ratio",
        "mean_timing_ratio",
        "weighted_timing_ratio",
        "total_baseline_average_elapsed_ns",
        "total_fse_average_elapsed_ns",
    ]
}

fn aggregate_value_fields(baseline: &BaselineAggregateSummary) -> Vec<String> {
    vec![
        baseline.baseline_name.clone(),
        baseline.baseline_label.clone(),
        baseline.comparison_label.clone(),
        baseline.workload_count.to_string(),
        baseline.total_baseline_evaluated_records.to_string(),
        baseline.total_fse_reconstructed_records.to_string(),
        format_ratio(baseline.weighted_reconstruction_avoidance_ratio as f64),
        format_ratio(baseline.weighted_candidate_ratio as f64),
        format_ratio(baseline.mean_timing_ratio),
        format_ratio(baseline.weighted_timing_ratio),
        baseline
            .total_baseline_average_elapsed
            .as_nanos()
            .to_string(),
        baseline.total_fse_average_elapsed.as_nanos().to_string(),
    ]
}

fn workload_header_fields() -> Vec<&'static str> {
    vec![
        "baseline_name",
        "baseline_label",
        "comparison_label",
        "workload_name",
        "baseline_evaluated_records",
        "baseline_matched_records",
        "fse_visited_nodes",
        "fse_retained_leaves",
        "fse_reconstructed_records",
        "fse_matched_records",
        "avoided_reconstructions",
        "reconstruction_avoidance_ratio",
        "candidate_ratio",
        "retained_leaf_ratio",
        "record_pruning_efficiency",
        "leaf_pruning_efficiency",
        "single_run_timing_ratio",
        "average_timing_ratio",
        "baseline_elapsed_ns",
        "fse_elapsed_ns",
        "baseline_average_elapsed_ns",
        "fse_average_elapsed_ns",
    ]
}

fn header_fields_with_metadata(header_fields: Vec<&'static str>) -> Vec<&'static str> {
    let mut fields = metadata_header_fields();
    fields.extend(header_fields);
    fields
}

fn value_fields_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    value_fields: Vec<String>,
) -> Vec<String> {
    let mut fields = metadata_value_fields(metadata);
    fields.extend(value_fields);
    fields
}

fn csv_document<I>(header_fields: Vec<&'static str>, value_rows: I) -> String
where
    I: IntoIterator<Item = Vec<String>>,
{
    let mut rows = Vec::new();

    rows.push(csv_row(header_fields));

    for value_row in value_rows {
        rows.push(csv_row(value_row));
    }

    rows.join("\n")
}

fn csv_row<I, S>(fields: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fields
        .into_iter()
        .map(|field| escape_csv_field(field.as_ref()))
        .collect::<Vec<String>>()
        .join(",")
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}

fn format_ratio(value: f64) -> String {
    format!("{:.6}", value)
}
