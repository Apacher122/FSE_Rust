//! Selectivity summary report types.

use crate::math::Scalar;

use super::bucket::SelectivityBucket;

/// Summary of workloads that fall into one selectivity bucket.
///
/// # Runtime Role
///
/// `SelectivityBucketSummary` aggregates reconstruction, pruning, and timing
/// behavior for workloads with similar candidate ratios.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SelectivityBucketSummary {
    /// Selectivity bucket represented by this summary.
    pub bucket: SelectivityBucket,

    /// Number of workloads in the bucket.
    pub workload_count: usize,

    /// Total baseline record evaluations across workloads in the bucket.
    pub total_baseline_evaluated_records: usize,

    /// Total FSE reconstructed records across workloads in the bucket.
    pub total_fse_reconstructed_records: usize,

    /// Total baseline evaluations avoided by FSE reconstruction.
    pub total_avoided_reconstructions: usize,

    /// Average candidate ratio across workloads in the bucket.
    pub average_candidate_ratio: Scalar,

    /// Candidate ratio weighted by total baseline evaluated records.
    pub weighted_candidate_ratio: Scalar,

    /// Average reconstruction avoidance ratio across workloads in the bucket.
    pub average_reconstruction_avoidance_ratio: Scalar,

    /// Reconstruction avoidance ratio weighted by total baseline evaluated records.
    pub weighted_reconstruction_avoidance_ratio: Scalar,

    /// Arithmetic mean of per-workload average timing ratios.
    pub mean_timing_ratio: f64,
}

/// Selectivity-bucketed summary across workload comparisons.
///
/// # Runtime Role
///
/// `SelectivityBucketedWorkloadSummary` provides a compact view of how benchmark
/// behavior changes as FSE reconstructs more or fewer records.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SelectivityBucketedWorkloadSummary {
    /// Non-empty bucket summaries in stable bucket order.
    pub bucket_summaries: Vec<SelectivityBucketSummary>,
}

impl SelectivityBucketedWorkloadSummary {
    /// Returns a summary for the requested bucket when present.
    pub fn bucket_summary(&self, bucket: SelectivityBucket) -> Option<&SelectivityBucketSummary> {
        self.bucket_summaries
            .iter()
            .find(|summary| summary.bucket == bucket)
    }

    /// Returns the total number of workloads represented by all buckets.
    pub fn total_workload_count(&self) -> usize {
        self.bucket_summaries
            .iter()
            .map(|summary| summary.workload_count)
            .sum()
    }

    /// Returns whether no workloads were summarized.
    pub fn is_empty(&self) -> bool {
        self.bucket_summaries.is_empty()
    }
}
