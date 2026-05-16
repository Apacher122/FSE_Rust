//! Selectivity-bucketed workload summaries.

use crate::benchmark::WorkloadComparisonSummary;
use crate::math::Scalar;

/// Stable selectivity bucket for workload-level benchmark summaries.
///
/// # Runtime Role
///
/// `SelectivityBucket` groups workloads by the fraction of baseline records that
/// FSE reconstructs. This makes benchmark behavior easier to interpret across
/// query shapes instead of reading each workload row manually.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectivityBucket {
    /// Workloads that reconstruct no records.
    Empty,

    /// Workloads that reconstruct at most one quarter of baseline records.
    Low,

    /// Workloads that reconstruct at most half of baseline records.
    Medium,

    /// Workloads that reconstruct less than the full baseline record count.
    High,

    /// Workloads that reconstruct the full baseline record count or more.
    Full,
}

impl SelectivityBucket {
    /// Returns the bucket for a candidate ratio.
    ///
    /// # Runtime Role
    ///
    /// This classifier is intentionally based on candidate ratio rather than
    /// match ratio because reconstruction volume is the direct cost controlled
    /// by FSE pruning.
    pub fn from_candidate_ratio(candidate_ratio: Scalar) -> Self {
        if candidate_ratio <= 0.0 {
            return Self::Empty;
        }

        if candidate_ratio <= 0.25 {
            return Self::Low;
        }

        if candidate_ratio <= 0.50 {
            return Self::Medium;
        }

        if candidate_ratio < 1.0 {
            return Self::High;
        }

        Self::Full
    }

    /// Returns the stable label used in benchmark output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Full => "full",
        }
    }
}

impl Default for SelectivityBucket {
    fn default() -> Self {
        Self::Empty
    }
}

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

/// Renders a selectivity-bucketed workload summary for terminal output.
///
/// # Runtime Role
///
/// This renderer gives benchmark runs a compact view of how reconstruction,
/// pruning, and timing behavior change across candidate-ratio bands.
pub fn render_selectivity_bucketed_workload_summary(
    summary: &SelectivityBucketedWorkloadSummary,
) -> String {
    if summary.is_empty() {
        return String::new();
    }

    let mut output = String::new();

    output.push_str("\nSelectivity Bucket Summary\n");
    output.push_str("--------------------------\n");
    output.push_str(
        "bucket | workloads | baseline records | fse records | avoided | avg candidate | weighted candidate | avg avoidance | weighted avoidance | mean timing\n",
    );

    for bucket_summary in &summary.bucket_summaries {
        output.push_str(&render_selectivity_bucket_summary_row(bucket_summary));
    }

    output
}

fn render_selectivity_bucket_summary_row(summary: &SelectivityBucketSummary) -> String {
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {}\n",
        summary.bucket.label(),
        summary.workload_count,
        summary.total_baseline_evaluated_records,
        summary.total_fse_reconstructed_records,
        summary.total_avoided_reconstructions,
        format_scalar_ratio(summary.average_candidate_ratio),
        format_scalar_ratio(summary.weighted_candidate_ratio),
        format_scalar_ratio(summary.average_reconstruction_avoidance_ratio),
        format_scalar_ratio(summary.weighted_reconstruction_avoidance_ratio),
        format_f64_ratio(summary.mean_timing_ratio),
    )
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

    summary.weighted_candidate_ratio = ratio_or_zero(
        summary.total_fse_reconstructed_records,
        summary.total_baseline_evaluated_records,
    );

    summary.weighted_reconstruction_avoidance_ratio = ratio_or_zero(
        summary.total_avoided_reconstructions,
        summary.total_baseline_evaluated_records,
    );

    summary
}

fn ratio_or_zero(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        0.0
    } else {
        numerator as Scalar / denominator as Scalar
    }
}

fn format_scalar_ratio(value: Scalar) -> String {
    format!("{:.6}", value)
}

fn format_f64_ratio(value: f64) -> String {
    format!("{:.6}", value)
}
