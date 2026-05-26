//! Single-baseline suite terminal rendering.

use std::fmt::Write;

use crate::benchmark::BenchmarkSuiteReport;

use super::duration::format_duration_ascii;

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

    render_aggregate_metrics(report, &mut output);

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
        "total baseline average elapsed: {}",
        format_duration_ascii(aggregate.total_baseline_average_elapsed)
    )
    .unwrap();
    writeln!(
        output,
        "total FSE average elapsed: {}",
        format_duration_ascii(aggregate.total_fse_average_elapsed)
    )
    .unwrap();
    writeln!(
        output,
        "mean baseline average elapsed: {}",
        format_duration_ascii(aggregate.mean_baseline_average_elapsed)
    )
    .unwrap();
    writeln!(
        output,
        "mean FSE average elapsed: {}",
        format_duration_ascii(aggregate.mean_fse_average_elapsed)
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
