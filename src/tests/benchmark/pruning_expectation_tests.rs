//! Workload-level pruning expectation tests.

use crate::benchmark::reports::RepeatedTimingConfig;
use crate::benchmark::{
    BaselineKind, BaselineRegistry, BenchmarkSuiteReport, WorkloadComparisonSummary,
    WorkloadPruningReport, run_benchmark_suite_with_registry,
};
use crate::math::Scalar;
use crate::tests::support::small_benchmark_fixture;

const RATIO_TOLERANCE: Scalar = 0.000001;

#[test]
fn pruning_reports_keep_workload_order_aligned_with_comparisons() {
    let report = small_flat_scan_suite_report();

    let comparison_names: Vec<&str> = report
        .comparisons
        .iter()
        .map(|summary| summary.workload_name.as_str())
        .collect();

    let pruning_names: Vec<&str> = report
        .pruning_reports
        .iter()
        .map(|report| report.workload_name.as_str())
        .collect();

    assert_eq!(pruning_names, comparison_names);
}

#[test]
fn empty_far_range_prunes_every_leaf_without_reconstruction() {
    let report = small_flat_scan_suite_report();
    let summary = workload_comparison_summary(&report, "empty_far_range");
    let pruning_report = workload_pruning_report(&report, "empty_far_range");

    let comparison = &summary.comparison;
    let pruning = &pruning_report.pruning;

    // empty ranges should stay boring no leaves no records
    assert_eq!(comparison.baseline_stats.evaluated_records, 60);
    assert_eq!(comparison.fse_stats.reconstructed_records, 0);
    assert_eq!(comparison.fse_stats.retained_leaves, 0);

    assert_ratio_eq(comparison.candidate_ratio, 0.0);
    assert_ratio_eq(comparison.retained_leaf_ratio, 0.0);
    assert_ratio_eq(pruning.record_pruning_efficiency, 1.0);
    assert_ratio_eq(pruning.leaf_pruning_efficiency, 1.0);
}

#[test]
fn full_dataset_range_reconstructs_every_record_and_retains_every_leaf() {
    let report = small_flat_scan_suite_report();
    let summary = workload_comparison_summary(&report, "full_dataset_range");
    let pruning_report = workload_pruning_report(&report, "full_dataset_range");

    let comparison = &summary.comparison;
    let pruning = &pruning_report.pruning;

    assert_eq!(comparison.baseline_stats.evaluated_records, 60);
    assert_eq!(
        comparison.fse_stats.reconstructed_records,
        comparison.baseline_stats.evaluated_records
    );

    assert_ratio_eq(comparison.candidate_ratio, 1.0);
    assert_ratio_eq(comparison.retained_leaf_ratio, 1.0);
    assert_ratio_eq(pruning.record_pruning_efficiency, 0.0);
    assert_ratio_eq(pruning.leaf_pruning_efficiency, 0.0);
}

#[test]
fn cluster_range_workload_reconstructs_only_a_candidate_subset() {
    let report = small_flat_scan_suite_report();
    let summary = workload_comparison_summary(&report, "cluster_range_000");
    let pruning_report = workload_pruning_report(&report, "cluster_range_000");

    let comparison = &summary.comparison;
    let pruning = &pruning_report.pruning;

    // this should hit part of the first cluster not the whole dataset
    assert!(comparison.fse_stats.reconstructed_records > 0);
    assert!(
        comparison.fse_stats.reconstructed_records < comparison.baseline_stats.evaluated_records
    );
    assert!(comparison.fse_stats.retained_leaves > 0);

    assert_ratio_between_zero_and_one(comparison.candidate_ratio);
    assert_ratio_between_zero_and_one(comparison.retained_leaf_ratio);
    assert_ratio_between_zero_and_one(pruning.record_pruning_efficiency);
    assert_ratio_between_zero_and_one(pruning.leaf_pruning_efficiency);
}

#[test]
fn cluster_boundary_workload_keeps_partial_pruning_behavior() {
    let report = small_flat_scan_suite_report();
    let summary = workload_comparison_summary(&report, "cluster_boundary_range");
    let pruning_report = workload_pruning_report(&report, "cluster_boundary_range");

    let comparison = &summary.comparison;
    let pruning = &pruning_report.pruning;

    // boundary cases catch weird tree pruning behvior first
    assert!(comparison.fse_stats.reconstructed_records > 0);
    assert!(
        comparison.fse_stats.reconstructed_records < comparison.baseline_stats.evaluated_records
    );
    assert!(comparison.fse_stats.retained_leaves > 0);

    assert_ratio_between_zero_and_one(comparison.candidate_ratio);
    assert_ratio_between_zero_and_one(comparison.retained_leaf_ratio);
    assert_ratio_between_zero_and_one(pruning.record_pruning_efficiency);
    assert_ratio_between_zero_and_one(pruning.leaf_pruning_efficiency);
}

fn small_flat_scan_suite_report() -> BenchmarkSuiteReport {
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

fn workload_comparison_summary<'a>(
    report: &'a BenchmarkSuiteReport,
    workload_name: &str,
) -> &'a WorkloadComparisonSummary {
    report
        .comparisons
        .iter()
        .find(|summary| summary.workload_name == workload_name)
        .unwrap_or_else(|| {
            panic!(
                "missing comparison summary for workload `{}`",
                workload_name
            )
        })
}

fn workload_pruning_report<'a>(
    report: &'a BenchmarkSuiteReport,
    workload_name: &str,
) -> &'a WorkloadPruningReport {
    report
        .pruning_reports
        .iter()
        .find(|report| report.workload_name == workload_name)
        .unwrap_or_else(|| panic!("missing pruning report for workload `{}`", workload_name))
}

fn assert_ratio_eq(actual: Scalar, expected: Scalar) {
    assert!(
        (actual - expected).abs() <= RATIO_TOLERANCE,
        "expected ratio {}, got {}",
        expected,
        actual
    );
}

fn assert_ratio_between_zero_and_one(value: Scalar) {
    assert!(
        value > 0.0 && value < 1.0,
        "expected ratio between 0 and 1, got {}",
        value
    );
}
