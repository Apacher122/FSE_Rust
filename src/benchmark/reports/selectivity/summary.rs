//! Selectivity summary construction.

use crate::benchmark::WorkloadComparisonSummary;
use crate::benchmark::math::scalar_ratio_or_zero;
use crate::math::Scalar;

use super::bucket::SelectivityBucket;
use super::report::{SelectivityBucketSummary, SelectivityBucketedWorkloadSummary};

/// Builds selectivity-bucketed summaries from workload comparison summaries.
///
/// # Runtime Role
///
/// This function groups workloads by candidate ratio and aggregates the same
/// core pruning and timing fields used by the benchmark reports.
pub fn summarize_workloads_by_selectivity(
    workload_summaries: &[WorkloadComparisonSummary],
) -> SelectivityBucketedWorkloadSummary {
    let mut bucket_summaries = Vec::new();

    for bucket in selectivity_bucket_order() {
        let bucket_workloads = workloads_for_bucket(workload_summaries, bucket);

        if bucket_workloads.is_empty() {
            continue;
        }

        // buckets stay fixed so output and tests dont jump around
        bucket_summaries.push(build_bucket_summary(bucket, &bucket_workloads));
    }

    SelectivityBucketedWorkloadSummary { bucket_summaries }
}

fn selectivity_bucket_order() -> [SelectivityBucket; 5] {
    [
        SelectivityBucket::Empty,
        SelectivityBucket::Low,
        SelectivityBucket::Medium,
        SelectivityBucket::High,
        SelectivityBucket::Full,
    ]
}

fn workloads_for_bucket<'a>(
    workload_summaries: &'a [WorkloadComparisonSummary],
    bucket: SelectivityBucket,
) -> Vec<&'a WorkloadComparisonSummary> {
    workload_summaries
        .iter()
        .filter(|summary| {
            SelectivityBucket::from_candidate_ratio(summary.comparison.candidate_ratio) == bucket
        })
        .collect()
}

fn build_bucket_summary(
    bucket: SelectivityBucket,
    workload_summaries: &[&WorkloadComparisonSummary],
) -> SelectivityBucketSummary {
    let workload_count = workload_summaries.len();

    let mut summary = SelectivityBucketSummary {
        bucket,
        workload_count,
        ..SelectivityBucketSummary::default()
    };

    let mut candidate_ratio_sum = 0.0;
    let mut reconstruction_avoidance_ratio_sum = 0.0;
    let mut timing_ratio_sum = 0.0;

    for workload_summary in workload_summaries {
        let comparison = &workload_summary.comparison;

        summary.total_baseline_evaluated_records += comparison.baseline_stats.evaluated_records;
        summary.total_fse_reconstructed_records += comparison.fse_stats.reconstructed_records;
        summary.total_avoided_reconstructions += comparison.avoided_reconstructions;

        candidate_ratio_sum += comparison.candidate_ratio;
        reconstruction_avoidance_ratio_sum += comparison.reconstruction_avoidance_ratio;
        timing_ratio_sum += comparison.average_timing_ratio;
    }

    summary.average_candidate_ratio = candidate_ratio_sum / workload_count as Scalar;
    summary.average_reconstruction_avoidance_ratio =
        reconstruction_avoidance_ratio_sum / workload_count as Scalar;
    summary.mean_timing_ratio = timing_ratio_sum / workload_count as f64;

    summary.weighted_candidate_ratio = scalar_ratio_or_zero(
        summary.total_fse_reconstructed_records,
        summary.total_baseline_evaluated_records,
    );

    summary.weighted_reconstruction_avoidance_ratio = scalar_ratio_or_zero(
        summary.total_avoided_reconstructions,
        summary.total_baseline_evaluated_records,
    );

    summary
}
