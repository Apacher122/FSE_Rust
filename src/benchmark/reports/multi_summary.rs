//! Multi-baseline aggregate summaries.

use std::time::Duration;

use crate::benchmark::{BaselineKind, MultiBaselineBenchmarkSuiteReport};
use crate::math::Scalar;

/// Compact aggregate summary for one baseline suite.
///
/// # Runtime Role
///
/// `BaselineAggregateSummary` extracts the headline aggregate metrics from one
/// baseline comparison report so multiple baselines can be compared side by side.
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineAggregateSummary {
    /// Baseline kind used for this summary.
    pub baseline_kind: BaselineKind,

    /// Stable baseline identifier.
    pub baseline_name: String,

    /// Human-readable baseline label.
    pub baseline_label: String,

    /// Human-readable comparison label.
    pub comparison_label: String,

    /// Number of workloads included in the baseline suite.
    pub workload_count: usize,

    /// Total records evaluated by the baseline.
    pub total_baseline_evaluated_records: usize,

    /// Total records reconstructed by FSE.
    pub total_fse_reconstructed_records: usize,

    /// Weighted reconstruction avoidance ratio across all workload records.
    pub weighted_reconstruction_avoidance_ratio: Scalar,

    /// Weighted candidate ratio across all workload records.
    pub weighted_candidate_ratio: Scalar,

    /// Arithmetic mean of per-workload average timing ratios.
    pub mean_timing_ratio: f64,

    /// Timing ratio computed from aggregate average elapsed totals.
    pub weighted_timing_ratio: f64,

    /// Sum of per-workload baseline average elapsed times.
    pub total_baseline_average_elapsed: Duration,

    /// Sum of per-workload FSE average elapsed times.
    pub total_fse_average_elapsed: Duration,
}

/// Compact summary across all baseline suites.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultiBaselineAggregateSummary {
    /// Per-baseline aggregate summaries.
    pub baseline_summaries: Vec<BaselineAggregateSummary>,
}

impl MultiBaselineAggregateSummary {
    /// Returns the baseline with the highest weighted timing ratio.
    ///
    /// # Notes
    ///
    /// A higher timing ratio means the baseline took more time relative to FSE
    /// for the aggregate workload. It does not mean the baseline itself is
    /// faster.
    pub fn highest_weighted_timing_ratio(&self) -> Option<&BaselineAggregateSummary> {
        self.baseline_summaries.iter().max_by(|left, right| {
            left.weighted_timing_ratio
                .partial_cmp(&right.weighted_timing_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

/// Builds a compact aggregate summary from a multi-baseline benchmark report.
pub fn summarize_multi_baseline_aggregates(
    report: &MultiBaselineBenchmarkSuiteReport,
) -> MultiBaselineAggregateSummary {
    let baseline_summaries = report
        .baseline_reports
        .iter()
        .map(|baseline_report| {
            let aggregate = &baseline_report.report.aggregate;

            let labels = baseline_report
                .report
                .comparisons
                .first()
                .map(|summary| summary.comparison.labels.clone());

            let baseline_label = labels
                .as_ref()
                .map(|labels| labels.baseline_label.clone())
                .unwrap_or_else(|| baseline_report.baseline_name.clone());

            let comparison_label = labels
                .map(|labels| labels.comparison_label)
                .unwrap_or_else(|| format!("{} vs FSE", baseline_label));

            BaselineAggregateSummary {
                baseline_kind: baseline_report.baseline_kind,
                baseline_name: baseline_report.baseline_name.clone(),
                baseline_label,
                comparison_label,
                workload_count: aggregate.workload_count,
                total_baseline_evaluated_records: aggregate.total_baseline_evaluated_records,
                total_fse_reconstructed_records: aggregate.total_fse_reconstructed_records,
                weighted_reconstruction_avoidance_ratio: aggregate
                    .weighted_reconstruction_avoidance_ratio,
                weighted_candidate_ratio: aggregate.weighted_candidate_ratio,
                mean_timing_ratio: aggregate.mean_timing_ratio,
                weighted_timing_ratio: aggregate.weighted_timing_ratio,
                total_baseline_average_elapsed: aggregate.total_baseline_average_elapsed,
                total_fse_average_elapsed: aggregate.total_fse_average_elapsed,
            }
        })
        .collect();

    MultiBaselineAggregateSummary { baseline_summaries }
}
