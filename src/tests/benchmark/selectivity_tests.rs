//! Selectivity-bucketed workload summary tests.

use crate::benchmark::reports::RepeatedTimingConfig;
use crate::benchmark::{
    BaselineKind, BaselineRegistry, SelectivityBucket, SelectivityBucketedWorkloadSummary,
    render_selectivity_bucketed_workload_summary, run_benchmark_suite_with_registry,
    summarize_workloads_by_selectivity,
};
use crate::math::Scalar;
use crate::tests::support::small_benchmark_fixture;

const RATIO_TOLERANCE: Scalar = 0.000001;

#[test]
fn selectivity_bucket_classifies_candidate_ratios() {
    assert_eq!(
        SelectivityBucket::from_candidate_ratio(0.0),
        SelectivityBucket::Empty
    );
    assert_eq!(
        SelectivityBucket::from_candidate_ratio(0.01),
        SelectivityBucket::Low
    );
    assert_eq!(
        SelectivityBucket::from_candidate_ratio(0.25),
        SelectivityBucket::Low
    );
    assert_eq!(
        SelectivityBucket::from_candidate_ratio(0.50),
        SelectivityBucket::Medium
    );
    assert_eq!(
        SelectivityBucket::from_candidate_ratio(0.75),
        SelectivityBucket::High
    );
    assert_eq!(
        SelectivityBucket::from_candidate_ratio(1.0),
        SelectivityBucket::Full
    );
}

#[test]
fn selectivity_bucket_reports_stable_labels() {
    assert_eq!(SelectivityBucket::Empty.label(), "empty");
    assert_eq!(SelectivityBucket::Low.label(), "low");
    assert_eq!(SelectivityBucket::Medium.label(), "medium");
    assert_eq!(SelectivityBucket::High.label(), "high");
    assert_eq!(SelectivityBucket::Full.label(), "full");
}

#[test]
fn selectivity_summary_handles_empty_input() {
    let summary = summarize_workloads_by_selectivity(&[]);

    assert!(summary.is_empty());
    assert_eq!(summary.total_workload_count(), 0);
    assert!(summary.bucket_summary(SelectivityBucket::Empty).is_none());
}

#[test]
fn selectivity_summary_represents_every_workload_once() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);

    assert_eq!(summary.total_workload_count(), report.comparisons.len());
}

#[test]
fn selectivity_summary_includes_empty_and_full_buckets_for_small_fixture() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);

    assert!(summary.bucket_summary(SelectivityBucket::Empty).is_some());
    assert!(summary.bucket_summary(SelectivityBucket::Full).is_some());
}

#[test]
fn empty_selectivity_bucket_reports_zero_candidate_work() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);
    let empty_bucket = summary
        .bucket_summary(SelectivityBucket::Empty)
        .expect("small fixture should include empty workload bucket");

    // empty queries should not get mixed into low selectivity
    assert!(empty_bucket.workload_count > 0);
    assert_eq!(empty_bucket.total_fse_reconstructed_records, 0);
    assert_ratio_eq(empty_bucket.average_candidate_ratio, 0.0);
    assert_ratio_eq(empty_bucket.weighted_candidate_ratio, 0.0);
    assert_ratio_eq(empty_bucket.weighted_reconstruction_avoidance_ratio, 1.0);
}

#[test]
fn full_selectivity_bucket_reports_full_candidate_work() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);
    let full_bucket = summary
        .bucket_summary(SelectivityBucket::Full)
        .expect("small fixture should include full workload bucket");

    assert!(full_bucket.workload_count > 0);
    assert_eq!(
        full_bucket.total_fse_reconstructed_records,
        full_bucket.total_baseline_evaluated_records
    );
    assert_ratio_eq(full_bucket.average_candidate_ratio, 1.0);
    assert_ratio_eq(full_bucket.weighted_candidate_ratio, 1.0);
    assert_ratio_eq(full_bucket.weighted_reconstruction_avoidance_ratio, 0.0);
}

#[test]
fn selectivity_bucket_weighted_candidate_ratio_uses_record_totals() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);

    for bucket_summary in summary.bucket_summaries {
        let expected = bucket_summary.total_fse_reconstructed_records as Scalar
            / bucket_summary.total_baseline_evaluated_records as Scalar;

        assert_ratio_eq(bucket_summary.weighted_candidate_ratio, expected);
    }
}

#[test]
fn selectivity_bucketed_summary_default_is_empty() {
    let summary = SelectivityBucketedWorkloadSummary::default();

    assert!(summary.is_empty());
    assert_eq!(summary.total_workload_count(), 0);
}

#[test]
fn render_selectivity_bucketed_workload_summary_returns_empty_string_for_empty_summary() {
    let summary = SelectivityBucketedWorkloadSummary::default();

    // this makes empty rendering safe for callers that append blindly
    assert_eq!(render_selectivity_bucketed_workload_summary(&summary), "");
}

#[test]
fn render_selectivity_bucketed_workload_summary_includes_header() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);

    let output = render_selectivity_bucketed_workload_summary(&summary);

    assert!(output.contains("Selectivity Bucket Summary"));
    assert!(output.contains("bucket | workloads | baseline records | fse records"));
}

#[test]
fn render_selectivity_bucketed_workload_summary_includes_bucket_rows() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);

    let output = render_selectivity_bucketed_workload_summary(&summary);

    assert!(output.contains("empty |"));
    assert!(output.contains("full |"));
}

#[test]
fn render_selectivity_bucketed_workload_summary_uses_fixed_ratio_precision() {
    let report = small_flat_scan_suite_report();
    let summary = summarize_workloads_by_selectivity(&report.comparisons);

    let output = render_selectivity_bucketed_workload_summary(&summary);

    assert!(output.contains("1.000000"));
    assert!(output.contains("0.000000"));
}

fn small_flat_scan_suite_report() -> crate::benchmark::BenchmarkSuiteReport {
    let fixture = small_benchmark_fixture();

    run_benchmark_suite_with_registry(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &RepeatedTimingConfig::new(1),
        &BaselineRegistry::new(),
        BaselineKind::FlatScan,
    )
}

fn assert_ratio_eq(actual: Scalar, expected: Scalar) {
    assert!(
        (actual - expected).abs() <= RATIO_TOLERANCE,
        "expected ratio {}, got {}",
        expected,
        actual
    );
}
