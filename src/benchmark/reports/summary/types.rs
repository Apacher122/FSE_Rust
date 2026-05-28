//! Workload summary report types.

use std::time::Duration;

use crate::benchmark::QueryComparisonReport;
use crate::math::Scalar;

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
