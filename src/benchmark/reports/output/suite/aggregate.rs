//! Aggregate suite terminal rendering.

use std::fmt::Write;

use crate::benchmark::BenchmarkSuiteReport;

use super::super::duration::format_duration_ascii;

pub(super) fn render_aggregate_metrics(report: &BenchmarkSuiteReport, output: &mut String) {
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
