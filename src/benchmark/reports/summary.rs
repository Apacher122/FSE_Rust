//! Workload comparison summaries.

use std::time::Duration;

use crate::benchmark::{
    QueryComparisonReport, QueryWorkloadCase, compare_query_execution, duration_ratio,
};
use crate::math::{Scalar, Vector};
use crate::storage::FSEIndex;

/// Comparison result for a named workload case.
///
/// # Runtime Role
///
/// `WorkloadComparisonSummary` pairs a workload name with the comparison report
/// produced by running FSE and baseline execution against the same query.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkloadComparisonSummary {
    /// Human-readable workload name.
    pub workload_name: String,

    /// Side-by-side execution comparison for the workload.
    pub comparison: QueryComparisonReport,
}

/// Aggregate metrics across a group of workload comparisons.
///
/// # Runtime Role
///
/// `AggregateWorkloadMetrics` provides a compact summary of query execution work
/// across multiple workload cases. This is useful for demos, benchmark reports,
/// and regression tracking.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AggregateWorkloadMetrics {
    /// Number of workload cases included in the aggregate.
    pub workload_count: usize,

    /// Total records evaluated by the baseline.
    pub total_baseline_evaluated_records: usize,

    /// Total hierarchy nodes visited by FSE.
    pub total_fse_visited_nodes: usize,

    /// Total leaf partitions retained by FSE.
    pub total_fse_retained_leaves: usize,

    /// Total records reconstructed by FSE.
    pub total_fse_reconstructed_records: usize,

    /// Total records matched by FSE.
    pub total_fse_matched_records: usize,

    /// Total baseline record evaluations avoided by FSE reconstruction.
    pub total_avoided_reconstructions: usize,

    /// Average reconstruction avoidance ratio across workload cases.
    pub average_reconstruction_avoidance_ratio: Scalar,

    /// Average candidate ratio across workload cases.
    pub average_candidate_ratio: Scalar,

    /// Average retained leaf ratio across workload cases.
    pub average_retained_leaf_ratio: Scalar,

    /// Weighted reconstruction avoidance ratio across all workload records.
    pub weighted_reconstruction_avoidance_ratio: Scalar,

    /// Weighted candidate ratio across all workload records.
    pub weighted_candidate_ratio: Scalar,

    /// Sum of per-workload baseline average elapsed times.
    pub total_baseline_average_elapsed: Duration,

    /// Sum of per-workload FSE average elapsed times.
    pub total_fse_average_elapsed: Duration,

    /// Mean baseline average elapsed time per workload.
    pub mean_baseline_average_elapsed: Duration,

    /// Mean FSE average elapsed time per workload.
    pub mean_fse_average_elapsed: Duration,

    /// Arithmetic mean of per-workload average timing ratios.
    pub mean_timing_ratio: f64,

    /// Timing ratio computed from aggregate average elapsed totals.
    pub weighted_timing_ratio: f64,
}

/// Runs all workload cases and returns comparison summaries.
///
/// # Runtime Role
///
/// This function is used by demos and future benchmark harnesses to evaluate a
/// stable set of query workloads against a fixed dataset and FSE index.
///
/// # Panics
///
/// Panics if any FSE query result differs from the baseline result.
pub fn summarize_workload_comparisons(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
) -> Vec<WorkloadComparisonSummary> {
    workloads
        .iter()
        .map(|workload| WorkloadComparisonSummary {
            workload_name: workload.name.clone(),
            comparison: compare_query_execution(index, points, &workload.query),
        })
        .collect()
}

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

    aggregate.weighted_reconstruction_avoidance_ratio = ratio_or_zero(
        aggregate.total_avoided_reconstructions,
        aggregate.total_baseline_evaluated_records,
    );

    aggregate.weighted_candidate_ratio = ratio_or_zero(
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

fn ratio_or_zero(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        0.0
    } else {
        numerator as Scalar / denominator as Scalar
    }
}

fn duration_div(duration: Duration, divisor: usize) -> Duration {
    if divisor == 0 {
        return Duration::ZERO;
    }

    // Keep duration averaging explicit to match the benchmark timing helper.
    Duration::from_secs_f64(duration.as_secs_f64() / divisor as f64)
}
