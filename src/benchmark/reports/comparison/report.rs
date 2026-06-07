//! Query comparison report type.

use crate::benchmark::baselines::{
    BaselineComparisonLabels, BaselineFootprintMetrics, BaselineQueryStats,
};
use crate::benchmark::reports::timing::{
    RepeatedComparisonTimingReport, RepeatedTimingReport, TimingReport,
};
use crate::math::Scalar;
use crate::query::QueryExecutionStats;
use std::time::Duration;

/// Side-by-side report comparing FSE query execution with baseline execution.
///
/// # Runtime Role
///
/// `QueryComparisonReport` is intended for early correctness and performance
/// analysis. It compares logical execution work and lightweight elapsed timing
/// between a baseline query path and the FSE query path.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryComparisonReport {
    /// Human-readable labels for this comparison.
    pub labels: BaselineComparisonLabels,

    /// Human-readable baseline name.
    pub baseline_name: String,

    /// Logical scalar footprint metrics for the baseline structure.
    pub baseline_footprint: BaselineFootprintMetrics,

    /// Statistics from the baseline execution path.
    pub baseline_stats: BaselineQueryStats,

    /// Statistics from the FSE owned-result execution path.
    pub fse_stats: QueryExecutionStats,

    /// Statistics from the FSE count-only execution path.
    pub count_only_stats: QueryExecutionStats,

    /// Statistics from the FSE reference-result execution path.
    pub reference_stats: QueryExecutionStats,

    /// Statistics from the FSE reusable owned-result execution path.
    pub reusable_owned_stats: QueryExecutionStats,

    /// Wall-clock timing measurements for one execution of both paths.
    pub timing: TimingReport,

    /// Repeated timing measurements for baseline and owned-result FSE execution.
    pub repeated_timing: RepeatedComparisonTimingReport,

    /// Repeated timing measurements for count-only FSE execution.
    pub count_only_repeated_timing: RepeatedTimingReport,

    /// Repeated timing measurements for reference-result FSE execution.
    pub reference_repeated_timing: RepeatedTimingReport,

    /// Repeated timing measurements for reusable owned-result FSE execution.
    pub reusable_owned_repeated_timing: RepeatedTimingReport,

    /// Estimated average elapsed time spent above count-only execution.
    pub estimated_owned_result_overhead: Duration,

    /// Estimated average elapsed time spent above reference-result execution.
    pub estimated_owned_vs_reference_overhead: Duration,

    /// Estimated average elapsed time spent above reusable owned-result execution.
    pub estimated_fresh_vs_reusable_owned_overhead: Duration,

    /// Average owned-result FSE elapsed divided by average count-only FSE elapsed.
    pub count_only_speedup_ratio: f64,

    /// Average owned-result FSE elapsed divided by average reference-result FSE elapsed.
    pub reference_result_speedup_ratio: f64,

    /// Average fresh owned-result FSE elapsed divided by average reusable owned-result FSE elapsed.
    pub reusable_owned_result_speedup_ratio: f64,

    /// Single-run timing ratio computed as baseline elapsed divided by FSE elapsed.
    pub single_run_timing_ratio: f64,

    /// Average timing ratio computed as baseline average elapsed divided by FSE average elapsed.
    pub average_timing_ratio: f64,

    /// Number of records avoided by FSE reconstruction relative to baseline evaluation.
    pub avoided_reconstructions: usize,

    /// Fraction of baseline record evaluations avoided by FSE reconstruction.
    pub reconstruction_avoidance_ratio: Scalar,

    /// Fraction of total records reconstructed by FSE.
    pub candidate_ratio: Scalar,

    /// Fraction of leaf partitions retained by FSE.
    pub retained_leaf_ratio: Scalar,
}
