//! Terminal output rendering for benchmark reports.

use std::fmt::Write;

use crate::benchmark::{BenchmarkSuiteReport, MultiBaselineAggregateSummary};
use crate::build::{IndexStructureMetrics, IndexValidationReport};
use crate::query::QueryExecutionMode;

/// Header information printed before benchmark reports.
///
/// # Runtime Role
///
/// `BenchmarkRunOverview` keeps benchmark run metadata separate from the binary
/// entry point so terminal rendering can be reused and tested independently.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkRunOverview {
    /// Number of records in the selected dataset.
    pub dataset_records: usize,

    /// Number of nodes in the constructed FSE index.
    pub index_nodes: usize,

    /// Number of workload cases in the benchmark run.
    pub workloads: usize,

    /// Comma-separated baseline names selected for the run.
    pub baselines: String,

    /// Number of repeated timing iterations.
    pub timing_iterations: usize,

    /// Target FSE leaf size used during construction.
    pub target_leaf_size: usize,

    /// Maximum FSE leaf size used during construction.
    pub max_leaf_size: usize,

    /// Maximum FSE build depth used during construction.
    pub max_depth: usize,

    /// FSE query execution mode used by the benchmark run.
    pub fse_execution_mode: QueryExecutionMode,

    /// Minimum retained-leaf count required before parallel FSE mode uses Rayon.
    pub fse_parallel_min_retained_leaves: usize,

    /// Structural metrics for the constructed FSE index.
    pub index_structure: IndexStructureMetrics,

    /// Validation report for the constructed FSE index.
    pub validation: IndexValidationReport,
}

impl BenchmarkRunOverview {
    /// Returns the user-facing name for the configured FSE execution mode.
    pub fn fse_execution_mode_name(&self) -> &'static str {
        match self.fse_execution_mode {
            QueryExecutionMode::Serial => "serial",
            QueryExecutionMode::Parallel => "parallel",
        }
    }
}

/// Renders the benchmark run overview.
pub fn render_benchmark_overview(overview: &BenchmarkRunOverview) -> String {
    let mut output = String::new();

    writeln!(output, "FSE benchmark suite").unwrap();
    writeln!(output, "===================").unwrap();
    writeln!(output).unwrap();

    writeln!(output, "Dataset records: {}", overview.dataset_records).unwrap();
    writeln!(output, "Index nodes: {}", overview.index_nodes).unwrap();
    writeln!(
        output,
        "Leaf nodes: {}",
        overview.index_structure.leaf_count
    )
    .unwrap();
    writeln!(
        output,
        "Internal nodes: {}",
        overview.index_structure.internal_node_count
    )
    .unwrap();
    writeln!(output, "Workloads: {}", overview.workloads).unwrap();
    writeln!(output, "Baselines: {}", overview.baselines).unwrap();
    writeln!(output, "Timing iterations: {}", overview.timing_iterations).unwrap();
    writeln!(output, "Target leaf size: {}", overview.target_leaf_size).unwrap();
    writeln!(output, "Max leaf size: {}", overview.max_leaf_size).unwrap();
    writeln!(output, "Max build depth: {}", overview.max_depth).unwrap();
    writeln!(
        output,
        "Max leaf cardinality: {}",
        overview.index_structure.max_leaf_cardinality
    )
    .unwrap();
    writeln!(
        output,
        "Average leaf cardinality: {:.2}",
        overview.index_structure.average_leaf_cardinality
    )
    .unwrap();
    writeln!(
        output,
        "Total leaf volume: {:.2}",
        overview.index_structure.total_leaf_volume
    )
    .unwrap();
    writeln!(
        output,
        "Index density: {:.2}",
        overview.index_structure.index_density
    )
    .unwrap();
    writeln!(
        output,
        "Zero-volume leaves: {}",
        overview.index_structure.zero_volume_leaf_count
    )
    .unwrap();
    writeln!(
        output,
        "FSE execution: {}",
        overview.fse_execution_mode_name()
    )
    .unwrap();
    writeln!(
        output,
        "FSE parallel min leaves: {}",
        overview.fse_parallel_min_retained_leaves
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "Timing ratio meaning: baseline elapsed / FSE elapsed"
    )
    .unwrap();
    writeln!(output, "  above 1.0 means FSE measured faster for that run").unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "Index validation: {}",
        overview.validation.is_valid()
    )
    .unwrap();
    writeln!(
        output,
        "  leaf cardinality valid: {}",
        overview.validation.leaf_cardinality_valid
    )
    .unwrap();
    writeln!(
        output,
        "  hierarchy topology valid: {}",
        overview.validation.hierarchy_topology_valid
    )
    .unwrap();
    writeln!(
        output,
        "  parent-child bounds valid: {}",
        overview.validation.parent_child_bounds_valid
    )
    .unwrap();
    writeln!(output).unwrap();

    output
}

/// Renders a named baseline suite section.
pub fn render_named_baseline_suite_report(
    baseline_name: &str,
    report: &BenchmarkSuiteReport,
) -> String {
    let mut output = String::new();

    writeln!(output, "Baseline suite: {}", baseline_name).unwrap();
    writeln!(output, "----------------").unwrap();
    output.push_str(&render_suite_report(report));
    writeln!(output).unwrap();

    output
}

/// Renders a benchmark suite report for one baseline.
pub fn render_suite_report(report: &BenchmarkSuiteReport) -> String {
    let mut output = String::new();

    for (summary, pruning_report) in report.comparisons.iter().zip(&report.pruning_reports) {
        let comparison = &summary.comparison;
        let pruning = &pruning_report.pruning;

        writeln!(output, "Workload: {}", summary.workload_name).unwrap();
        writeln!(output, "Comparison: {}", comparison.labels.comparison_label).unwrap();
        writeln!(output, "Stats:").unwrap();
        writeln!(output, "  baseline: {}", comparison.labels.baseline_label).unwrap();
        writeln!(
            output,
            "  baseline evaluated records: {}",
            comparison.baseline_stats.evaluated_records
        )
        .unwrap();
        writeln!(
            output,
            "  baseline elapsed: {:?}",
            comparison.timing.baseline_elapsed
        )
        .unwrap();
        writeln!(
            output,
            "  baseline average elapsed: {:?}",
            comparison.repeated_timing.baseline.average_elapsed
        )
        .unwrap();
        writeln!(
            output,
            "  FSE visited nodes: {}",
            comparison.fse_stats.visited_nodes
        )
        .unwrap();
        writeln!(output, "  FSE elapsed: {:?}", comparison.timing.fse_elapsed).unwrap();
        writeln!(
            output,
            "  FSE average elapsed: {:?}",
            comparison.repeated_timing.fse.average_elapsed
        )
        .unwrap();
        writeln!(
            output,
            "  single-run timing ratio: {:.2}",
            comparison.single_run_timing_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  average timing ratio: {:.2}",
            comparison.average_timing_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  FSE retained leaves: {}",
            comparison.fse_stats.retained_leaves
        )
        .unwrap();
        writeln!(
            output,
            "  retained leaf ratio: {:.2}",
            comparison.retained_leaf_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  leaf pruning efficiency: {:.2}",
            pruning.leaf_pruning_efficiency
        )
        .unwrap();
        writeln!(
            output,
            "  FSE reconstructed records: {}",
            comparison.fse_stats.reconstructed_records
        )
        .unwrap();
        writeln!(
            output,
            "  candidate ratio: {:.2}",
            comparison.candidate_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  record pruning efficiency: {:.2}",
            pruning.record_pruning_efficiency
        )
        .unwrap();
        writeln!(
            output,
            "  matched records: {}",
            comparison.fse_stats.matched_records
        )
        .unwrap();
        writeln!(
            output,
            "  avoided reconstructions: {}",
            comparison.avoided_reconstructions
        )
        .unwrap();
        writeln!(
            output,
            "  reconstruction avoidance ratio: {:.2}",
            comparison.reconstruction_avoidance_ratio
        )
        .unwrap();
        writeln!(output).unwrap();
    }

    render_aggregate_metrics(report, &mut output);

    output
}

/// Renders a compact multi-baseline aggregate summary.
pub fn render_multi_baseline_summary(summary: &MultiBaselineAggregateSummary) -> String {
    let mut output = String::new();

    writeln!(output, "Multi-baseline aggregate summary").unwrap();
    writeln!(output, "--------------------------------").unwrap();

    for baseline in &summary.baseline_summaries {
        writeln!(output, "Baseline: {}", baseline.baseline_label).unwrap();
        writeln!(output, "Comparison: {}", baseline.comparison_label).unwrap();
        writeln!(output, "  workloads: {}", baseline.workload_count).unwrap();
        writeln!(
            output,
            "  total baseline evaluated records: {}",
            baseline.total_baseline_evaluated_records
        )
        .unwrap();
        writeln!(
            output,
            "  total FSE reconstructed records: {}",
            baseline.total_fse_reconstructed_records
        )
        .unwrap();
        writeln!(
            output,
            "  weighted reconstruction avoidance ratio: {:.2}",
            baseline.weighted_reconstruction_avoidance_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  weighted candidate ratio: {:.2}",
            baseline.weighted_candidate_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  mean timing ratio: {:.2}",
            baseline.mean_timing_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  weighted timing ratio: {:.2}",
            baseline.weighted_timing_ratio
        )
        .unwrap();
        writeln!(
            output,
            "  total baseline average elapsed: {:?}",
            baseline.total_baseline_average_elapsed
        )
        .unwrap();
        writeln!(
            output,
            "  total FSE average elapsed: {:?}",
            baseline.total_fse_average_elapsed
        )
        .unwrap();
        writeln!(output).unwrap();
    }

    if let Some(highest) = summary.highest_weighted_timing_ratio() {
        writeln!(
            output,
            "Highest weighted timing ratio: {} ({:.2})",
            highest.baseline_label, highest.weighted_timing_ratio
        )
        .unwrap();
    }

    output
}

fn render_aggregate_metrics(report: &BenchmarkSuiteReport, output: &mut String) {
    let aggregate = &report.aggregate;

    writeln!(output, "Aggregate workload metrics").unwrap();
    writeln!(output, "--------------------------").unwrap();
    writeln!(
        output,
        "total baseline evaluated records: {}",
        aggregate.total_baseline_evaluated_records
    )
    .unwrap();
    writeln!(
        output,
        "total FSE visited nodes: {}",
        aggregate.total_fse_visited_nodes
    )
    .unwrap();
    writeln!(
        output,
        "total FSE retained leaves: {}",
        aggregate.total_fse_retained_leaves
    )
    .unwrap();
    writeln!(
        output,
        "total FSE reconstructed records: {}",
        aggregate.total_fse_reconstructed_records
    )
    .unwrap();
    writeln!(
        output,
        "total FSE matched records: {}",
        aggregate.total_fse_matched_records
    )
    .unwrap();
    writeln!(
        output,
        "total avoided reconstructions: {}",
        aggregate.total_avoided_reconstructions
    )
    .unwrap();
    writeln!(
        output,
        "average reconstruction avoidance ratio: {:.2}",
        aggregate.average_reconstruction_avoidance_ratio
    )
    .unwrap();
    writeln!(
        output,
        "average candidate ratio: {:.2}",
        aggregate.average_candidate_ratio
    )
    .unwrap();
    writeln!(
        output,
        "average retained leaf ratio: {:.2}",
        aggregate.average_retained_leaf_ratio
    )
    .unwrap();
    writeln!(
        output,
        "weighted reconstruction avoidance ratio: {:.2}",
        aggregate.weighted_reconstruction_avoidance_ratio
    )
    .unwrap();
    writeln!(
        output,
        "weighted candidate ratio: {:.2}",
        aggregate.weighted_candidate_ratio
    )
    .unwrap();
    writeln!(
        output,
        "total baseline average elapsed: {:?}",
        aggregate.total_baseline_average_elapsed
    )
    .unwrap();
    writeln!(
        output,
        "total FSE average elapsed: {:?}",
        aggregate.total_fse_average_elapsed
    )
    .unwrap();
    writeln!(
        output,
        "mean baseline average elapsed: {:?}",
        aggregate.mean_baseline_average_elapsed
    )
    .unwrap();
    writeln!(
        output,
        "mean FSE average elapsed: {:?}",
        aggregate.mean_fse_average_elapsed
    )
    .unwrap();
    writeln!(
        output,
        "mean timing ratio: {:.2}",
        aggregate.mean_timing_ratio
    )
    .unwrap();
    writeln!(
        output,
        "weighted timing ratio: {:.2}",
        aggregate.weighted_timing_ratio
    )
    .unwrap();
}
