//! Multi-baseline terminal summary rendering.

use std::fmt::Write;

use crate::benchmark::MultiBaselineAggregateSummary;

use super::duration::format_duration_ascii;

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
            "  total baseline average elapsed: {}",
            format_duration_ascii(baseline.total_baseline_average_elapsed)
        )
        .unwrap();
        writeln!(
            output,
            "  total FSE average elapsed: {}",
            format_duration_ascii(baseline.total_fse_average_elapsed)
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
