//! Per-workload suite terminal rendering.

use std::fmt::Write;

use crate::benchmark::{WorkloadComparisonSummary, WorkloadPruningReport};

use super::super::duration::format_duration_ascii;

pub(super) fn render_workload_comparison(
    summary: &WorkloadComparisonSummary,
    pruning_report: &WorkloadPruningReport,
    output: &mut String,
) {
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
        "  baseline elapsed: {}",
        format_duration_ascii(comparison.timing.baseline_elapsed)
    )
    .unwrap();
    writeln!(
        output,
        "  baseline average elapsed: {}",
        format_duration_ascii(comparison.repeated_timing.baseline.average_elapsed)
    )
    .unwrap();
    writeln!(
        output,
        "  FSE visited nodes: {}",
        comparison.fse_stats.visited_nodes
    )
    .unwrap();
    writeln!(
        output,
        "  FSE elapsed: {}",
        format_duration_ascii(comparison.timing.fse_elapsed)
    )
    .unwrap();
    writeln!(
        output,
        "  FSE average elapsed: {}",
        format_duration_ascii(comparison.repeated_timing.fse.average_elapsed)
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
