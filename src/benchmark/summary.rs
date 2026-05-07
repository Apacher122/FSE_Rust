//! Workload comparison summaries.

use crate::benchmark::QueryComparisonReport;
use crate::math::Scalar;

/// Aggregate metrics across a group of workload comparisons.
///
/// # Runtime Role
///
/// `AggregateWorkloadMetrics` provides a compact summary of query execution work
/// across multiple workload cases. This is useful for demos, benchmark reports,
/// and regression tracking.
pub struct AggregateWorkloadMetrics {
    /// Number of workload cases included in the aggregate.
    pub workload_count: usize,
    /// Total records evaluated by the flat scan baseline.
    pub total_scan_evaluated_records: usize,
    /// Average reconstruction avoidance ratio across workload cases.
    pub average_reconstruction_avoidance_ratio: Scalar,
}

/// Aggregates workload comparison summaries into total and average metrics.
pub fn aggregate_workload_metrics(summaries: &[QueryComparisonReport]) -> AggregateWorkloadMetrics {
    let count = summaries.len();
    let total_scan: usize = summaries
        .iter()
        .map(|s| s.scan_stats.evaluated_records)
        .sum();
    let ratio_sum: Scalar = summaries
        .iter()
        .map(|s| s.reconstruction_avoidance_ratio)
        .sum();

    AggregateWorkloadMetrics {
        workload_count: count,
        total_scan_evaluated_records: total_scan,
        average_reconstruction_avoidance_ratio: if count == 0 {
            0.0
        } else {
            ratio_sum / count as Scalar
        },
    }
}
