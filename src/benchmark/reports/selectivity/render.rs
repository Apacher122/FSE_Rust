//! Selectivity summary terminal rendering.

use crate::benchmark::formatting::{format_f64_fixed_6, format_scalar_fixed_6};

use super::report::{SelectivityBucketSummary, SelectivityBucketedWorkloadSummary};

/// Renders a selectivity-bucketed workload summary for terminal output.
///
/// # Runtime Role
///
/// This renderer gives benchmark runs a compact view of how reconstruction,
/// pruning, and timing behavior change across candidate-ratio bands.
pub fn render_selectivity_bucketed_workload_summary(
    summary: &SelectivityBucketedWorkloadSummary,
) -> String {
    if summary.is_empty() {
        return String::new();
    }

    let mut output = String::new();

    output.push_str("\nSelectivity Bucket Summary\n");
    output.push_str("--------------------------\n");
    output.push_str(
        "bucket | workloads | baseline records | fse records | avoided | avg candidate | weighted candidate | avg avoidance | weighted avoidance | mean timing\n",
    );

    for bucket_summary in &summary.bucket_summaries {
        output.push_str(&render_selectivity_bucket_summary_row(bucket_summary));
    }

    output
}

fn render_selectivity_bucket_summary_row(summary: &SelectivityBucketSummary) -> String {
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {}\n",
        summary.bucket.label(),
        summary.workload_count,
        summary.total_baseline_evaluated_records,
        summary.total_fse_reconstructed_records,
        summary.total_avoided_reconstructions,
        format_scalar_fixed_6(summary.average_candidate_ratio),
        format_scalar_fixed_6(summary.weighted_candidate_ratio),
        format_scalar_fixed_6(summary.average_reconstruction_avoidance_ratio),
        format_scalar_fixed_6(summary.weighted_reconstruction_avoidance_ratio),
        format_f64_fixed_6(summary.mean_timing_ratio),
    )
}
