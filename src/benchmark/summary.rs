//! Workload comparison summaries.

use crate::benchmark::{QueryComparisonReport, QueryWorkloadCase, compare_query_execution};
use crate::math::{Scalar, Vector};
use crate::storage::FSEIndex;

/// Comparison result for a named workload case.
///
/// # Runtime Role
///
/// `WorkloadComparisonSummary` pairs a workload name with the comparison report
/// produced by running FSE and flat scan execution against the same query.
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

    /// Total records evaluated by the flat scan baseline.
    pub total_scan_evaluated_records: usize,

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
/// Panics if any FSE query result differs from the flat scan result.
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
/// The average reconstruction avoidance ratio is the arithmetic mean of the
/// per-workload ratios. It is intentionally separate from the total avoided
/// reconstruction ratio because both are useful for different reporting styles.
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

    let mut ratio_sum = 0.0;

    for summary in summaries {
        let comparison = &summary.comparison;

        aggregate.total_scan_evaluated_records += comparison.scan_stats.evaluated_records;
        aggregate.total_fse_visited_nodes += comparison.fse_stats.visited_nodes;
        aggregate.total_fse_retained_leaves += comparison.fse_stats.retained_leaves;
        aggregate.total_fse_reconstructed_records += comparison.fse_stats.reconstructed_records;
        aggregate.total_fse_matched_records += comparison.fse_stats.matched_records;
        aggregate.total_avoided_reconstructions += comparison.avoided_reconstructions;

        ratio_sum += comparison.reconstruction_avoidance_ratio;
    }

    aggregate.average_reconstruction_avoidance_ratio = ratio_sum / workload_count as Scalar;

    aggregate
}
