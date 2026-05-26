//! Shared terminal rendering helpers.

use crate::benchmark::{SelectivityBucket, WorkloadComparisonSummary};

pub(super) fn weakest_low_selectivity_workload(
    workload_summaries: &[WorkloadComparisonSummary],
) -> Option<&WorkloadComparisonSummary> {
    workload_summaries
        .iter()
        .filter(|summary| {
            SelectivityBucket::from_candidate_ratio(summary.comparison.candidate_ratio)
                == SelectivityBucket::Low
        })
        .min_by(|left, right| {
            left.comparison
                .average_timing_ratio
                .partial_cmp(&right.comparison.average_timing_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

pub(super) fn is_tree_baseline_name(baseline_name: &str) -> bool {
    baseline_name == "kd_tree" || baseline_name == "r_tree"
}
