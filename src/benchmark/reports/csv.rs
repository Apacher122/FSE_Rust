//! CSV export utilities for benchmark reports.

use std::fs;
use std::io;
use std::path::Path;

use super::multi_summary::{BaselineAggregateSummary, MultiBaselineAggregateSummary};
use super::output::BenchmarkRunOverview;
use super::selectivity::{SelectivityBucket, summarize_workloads_by_selectivity};
use crate::benchmark::baselines::BaselineKind;
use crate::benchmark::runner::{BaselineBenchmarkSuiteReport, MultiBaselineBenchmarkSuiteReport};
use crate::math::Scalar;

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

    /// Optional path for writing the low-selectivity tree-gap CSV.
    pub low_selectivity_gap_path: Option<String>,
}

impl BenchmarkCsvOutputConfig {
    /// Creates a CSV output configuration from optional paths.
    pub fn new(summary_path: Option<String>, workloads_path: Option<String>) -> Self {
        Self {
            summary_path,
            workloads_path,
            low_selectivity_gap_path: None,
        }
    }

    /// Returns whether no CSV output paths were configured.
    pub fn is_empty(&self) -> bool {
        self.summary_path.is_none()
            && self.workloads_path.is_none()
            && self.low_selectivity_gap_path.is_none()
    }

    /// Returns whether aggregate summary CSV output was configured.
    pub fn has_summary_output(&self) -> bool {
        self.summary_path.is_some()
    }

    /// Returns whether per-workload CSV output was configured.
    pub fn has_workload_output(&self) -> bool {
        self.workloads_path.is_some()
    }

    /// Returns whether low-selectivity tree-gap CSV output was configured.
    pub fn has_low_selectivity_gap_output(&self) -> bool {
        self.low_selectivity_gap_path.is_some()
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

    /// Sets the low-selectivity tree-gap CSV output path.
    pub fn set_low_selectivity_gap_path(&mut self, path: String) {
        // repeated flags use the same last-path-wins behavior
        self.low_selectivity_gap_path = Some(path);
    }
}

/// Metadata describing the benchmark run that produced a CSV export.
///
/// # Runtime Role
///
/// `BenchmarkCsvMetadata` makes exported benchmark rows self-describing so CSV
/// files can be compared later without relying on terminal output.
#[derive(Clone, Debug, PartialEq)]
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

    /// Target FSE leaf size used during construction.
    pub target_leaf_size: usize,

    /// Maximum FSE leaf size used during construction.
    pub max_leaf_size: usize,

    /// Maximum FSE build depth used during construction.
    pub max_depth: usize,

    /// FSE query execution mode used by the benchmark run.
    pub fse_execution_mode: String,

    /// Minimum retained-leaf count required before parallel FSE mode uses Rayon.
    pub fse_parallel_min_retained_leaves: usize,

    /// Number of leaf partitions in the constructed index.
    pub index_leaf_count: usize,

    /// Number of internal nodes in the constructed index.
    pub index_internal_node_count: usize,

    /// Total cardinality across leaf partitions.
    pub index_total_leaf_cardinality: usize,

    /// Minimum leaf cardinality.
    pub index_min_leaf_cardinality: usize,

    /// Maximum leaf cardinality.
    pub index_max_leaf_cardinality: usize,

    /// Average leaf cardinality.
    pub index_average_leaf_cardinality: Scalar,

    /// Sum of leaf bounding volumes.
    pub index_total_leaf_volume: Scalar,

    /// Average leaf bounding volume.
    pub index_average_leaf_volume: Scalar,

    /// Aggregate structural density across leaves.
    pub index_density: Scalar,

    /// Number of zero-volume leaves.
    pub index_zero_volume_leaf_count: usize,

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
            target_leaf_size: overview.target_leaf_size,
            max_leaf_size: overview.max_leaf_size,
            max_depth: overview.max_depth,
            fse_execution_mode: overview.fse_execution_mode_name().to_string(),
            fse_parallel_min_retained_leaves: overview.fse_parallel_min_retained_leaves,
            index_leaf_count: overview.index_structure.leaf_count,
            index_internal_node_count: overview.index_structure.internal_node_count,
            index_total_leaf_cardinality: overview.index_structure.total_leaf_cardinality,
            index_min_leaf_cardinality: overview.index_structure.min_leaf_cardinality,
            index_max_leaf_cardinality: overview.index_structure.max_leaf_cardinality,
            index_average_leaf_cardinality: overview.index_structure.average_leaf_cardinality,
            index_total_leaf_volume: overview.index_structure.total_leaf_volume,
            index_average_leaf_volume: overview.index_structure.average_leaf_volume,
            index_density: overview.index_structure.index_density,
            index_zero_volume_leaf_count: overview.index_structure.zero_volume_leaf_count,
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

/// Converts low-selectivity tree-gap summaries into CSV text.
///
/// # Runtime Role
///
/// This export mirrors the debug report's low-selectivity tree-gap section so
/// KD-tree and R-tree low-bucket behavior can be tracked across commits without
/// parsing terminal text.
pub fn multi_baseline_low_selectivity_gap_to_csv(
    report: &MultiBaselineBenchmarkSuiteReport,
) -> String {
    csv_document(
        low_selectivity_gap_header_fields(),
        low_selectivity_gap_value_rows(report),
    )
}

/// Converts low-selectivity tree-gap summaries into CSV text with run metadata.
pub fn multi_baseline_low_selectivity_gap_to_csv_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> String {
    csv_document(
        header_fields_with_metadata(low_selectivity_gap_header_fields()),
        low_selectivity_gap_value_rows(report)
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

fn low_selectivity_gap_value_rows(report: &MultiBaselineBenchmarkSuiteReport) -> Vec<Vec<String>> {
    let mut rows = Vec::new();

    for baseline_report in &report.baseline_reports {
        if !matches!(
            baseline_report.baseline_kind,
            BaselineKind::KdTree | BaselineKind::RTree
        ) {
            continue;
        }

        let selectivity_summary =
            summarize_workloads_by_selectivity(&baseline_report.report.comparisons);

        let Some(low_bucket) = selectivity_summary.bucket_summary(SelectivityBucket::Low) else {
            continue;
        };

        let labels = baseline_report
            .report
            .comparisons
            .first()
            .map(|summary| summary.comparison.labels.clone());

        let baseline_label = labels
            .as_ref()
            .map(|labels| labels.baseline_label.clone())
            .unwrap_or_else(|| baseline_report.baseline_name.clone());

        let comparison_label = labels
            .map(|labels| labels.comparison_label)
            .unwrap_or_else(|| format!("{} vs FSE", baseline_label));

        rows.push(vec![
            baseline_report.baseline_name.clone(),
            baseline_label,
            comparison_label,
            low_bucket.workload_count.to_string(),
            low_bucket.total_baseline_evaluated_records.to_string(),
            low_bucket.total_fse_reconstructed_records.to_string(),
            low_bucket.total_avoided_reconstructions.to_string(),
            format_ratio(low_bucket.average_candidate_ratio as f64),
            format_ratio(low_bucket.weighted_candidate_ratio as f64),
            format_ratio(low_bucket.average_reconstruction_avoidance_ratio as f64),
            format_ratio(low_bucket.weighted_reconstruction_avoidance_ratio as f64),
            format_ratio(low_bucket.mean_timing_ratio),
        ]);
    }

    rows
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
            comparison.count_only_stats.visited_nodes.to_string(),
            comparison.count_only_stats.retained_leaves.to_string(),
            comparison
                .count_only_stats
                .reconstructed_records
                .to_string(),
            comparison.count_only_stats.matched_records.to_string(),
            comparison
                .count_only_repeated_timing
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison
                .estimated_owned_result_overhead
                .as_nanos()
                .to_string(),
            format_ratio(comparison.count_only_speedup_ratio),
            (comparison.count_only_stats == comparison.fse_stats).to_string(),
            comparison.reference_stats.visited_nodes.to_string(),
            comparison.reference_stats.retained_leaves.to_string(),
            comparison.reference_stats.reconstructed_records.to_string(),
            comparison.reference_stats.matched_records.to_string(),
            comparison
                .reference_repeated_timing
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison
                .estimated_owned_vs_reference_overhead
                .as_nanos()
                .to_string(),
            format_ratio(comparison.reference_result_speedup_ratio),
            (comparison.reference_stats == comparison.count_only_stats).to_string(),
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
        "target_leaf_size",
        "max_leaf_size",
        "max_depth",
        "fse_execution_mode",
        "fse_parallel_min_retained_leaves",
        "index_leaf_count",
        "index_internal_node_count",
        "index_total_leaf_cardinality",
        "index_min_leaf_cardinality",
        "index_max_leaf_cardinality",
        "index_average_leaf_cardinality",
        "index_total_leaf_volume",
        "index_average_leaf_volume",
        "index_density",
        "index_zero_volume_leaf_count",
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
        metadata.target_leaf_size.to_string(),
        metadata.max_leaf_size.to_string(),
        metadata.max_depth.to_string(),
        metadata.fse_execution_mode.clone(),
        metadata.fse_parallel_min_retained_leaves.to_string(),
        metadata.index_leaf_count.to_string(),
        metadata.index_internal_node_count.to_string(),
        metadata.index_total_leaf_cardinality.to_string(),
        metadata.index_min_leaf_cardinality.to_string(),
        metadata.index_max_leaf_cardinality.to_string(),
        format_scalar(metadata.index_average_leaf_cardinality),
        format_scalar(metadata.index_total_leaf_volume),
        format_scalar(metadata.index_average_leaf_volume),
        format_scalar(metadata.index_density),
        metadata.index_zero_volume_leaf_count.to_string(),
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
        "count_only_visited_nodes",
        "count_only_retained_leaves",
        "count_only_reconstructed_records",
        "count_only_matched_records",
        "count_only_average_elapsed_ns",
        "estimated_owned_result_overhead_ns",
        "count_only_speedup_ratio",
        "count_only_stats_match_owned",
        "reference_visited_nodes",
        "reference_retained_leaves",
        "reference_reconstructed_records",
        "reference_matched_records",
        "reference_average_elapsed_ns",
        "estimated_owned_vs_reference_overhead_ns",
        "reference_result_speedup_ratio",
        "reference_stats_match_count_only",
    ]
}

fn low_selectivity_gap_header_fields() -> Vec<&'static str> {
    vec![
        "baseline_name",
        "baseline_label",
        "comparison_label",
        "low_workload_count",
        "low_baseline_evaluated_records",
        "low_fse_reconstructed_records",
        "low_avoided_reconstructions",
        "low_average_candidate_ratio",
        "low_weighted_candidate_ratio",
        "low_average_reconstruction_avoidance_ratio",
        "low_weighted_reconstruction_avoidance_ratio",
        "low_mean_timing_ratio",
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

fn format_scalar(value: Scalar) -> String {
    format!("{:.6}", value)
}

fn format_ratio(value: f64) -> String {
    format!("{:.6}", value)
}
