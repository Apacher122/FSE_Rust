//! Aggregate workload metrics.

use crate::benchmark::duration_ratio;
use crate::benchmark::math::{duration_div, scalar_ratio_or_zero};
use crate::math::Scalar;

use super::types::{AggregateWorkloadMetrics, WorkloadComparisonSummary};

/// Aggregates workload comparison summaries into total and average metrics.
///
/// # Runtime Role
///
/// This function converts per-workload comparison reports into a compact
/// benchmark-style summary.
///
/// # Notes
///
/// The average ratios are arithmetic means of the per-workload ratios.
/// Weighted ratios are computed from aggregate record counts or timing totals.
pub fn aggregate_workload_metrics(
    summaries: &[WorkloadComparisonSummary],
) -> AggregateWorkloadMetrics {
    let workload_count = summaries.len();

    if workload_count == 0 {
        return AggregateWorkloadMetrics::default();
    }

    let mut aggregate = AggregateWorkloadMetrics {
        workload_count,
        ..AggregateWorkloadMetrics::default()
    };

    let mut avoidance_ratio_sum = 0.0;
    let mut candidate_ratio_sum = 0.0;
    let mut retained_leaf_ratio_sum = 0.0;
    let mut timing_ratio_sum = 0.0;

    for summary in summaries {
        let comparison = &summary.comparison;

        aggregate.total_baseline_evaluated_records += comparison.baseline_stats.evaluated_records;
        aggregate.total_fse_visited_nodes += comparison.fse_stats.visited_nodes;
        aggregate.total_fse_retained_leaves += comparison.fse_stats.retained_leaves;
        aggregate.total_fse_reconstructed_records += comparison.fse_stats.reconstructed_records;
        aggregate.total_fse_matched_records += comparison.fse_stats.matched_records;
        aggregate.total_avoided_reconstructions += comparison.avoided_reconstructions;

        aggregate.total_baseline_average_elapsed +=
            comparison.repeated_timing.baseline.average_elapsed;
        aggregate.total_fse_average_elapsed += comparison.repeated_timing.fse.average_elapsed;

        avoidance_ratio_sum += comparison.reconstruction_avoidance_ratio;
        candidate_ratio_sum += comparison.candidate_ratio;
        retained_leaf_ratio_sum += comparison.retained_leaf_ratio;
        timing_ratio_sum += comparison.average_timing_ratio;
    }

    aggregate.average_reconstruction_avoidance_ratio =
        avoidance_ratio_sum / workload_count as Scalar;
    aggregate.average_candidate_ratio = candidate_ratio_sum / workload_count as Scalar;
    aggregate.average_retained_leaf_ratio = retained_leaf_ratio_sum / workload_count as Scalar;

    aggregate.weighted_reconstruction_avoidance_ratio = scalar_ratio_or_zero(
        aggregate.total_avoided_reconstructions,
        aggregate.total_baseline_evaluated_records,
    );

    aggregate.weighted_candidate_ratio = scalar_ratio_or_zero(
        aggregate.total_fse_reconstructed_records,
        aggregate.total_baseline_evaluated_records,
    );

    aggregate.mean_baseline_average_elapsed =
        duration_div(aggregate.total_baseline_average_elapsed, workload_count);
    aggregate.mean_fse_average_elapsed =
        duration_div(aggregate.total_fse_average_elapsed, workload_count);

    aggregate.mean_timing_ratio = timing_ratio_sum / workload_count as f64;
    aggregate.weighted_timing_ratio = duration_ratio(
        aggregate.total_baseline_average_elapsed,
        aggregate.total_fse_average_elapsed,
    );

    aggregate
}
