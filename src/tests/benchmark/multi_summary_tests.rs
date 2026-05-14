use crate::benchmark::{
    BaselineKind, BaselineRegistry, RepeatedTimingConfig, clustered_points_2d,
    clustered_workload_cases, run_multi_baseline_benchmark_suite,
    summarize_multi_baseline_aggregates,
};
use crate::build::{BuildConfig, FSEBuilder};

#[test]
fn multi_baseline_summary_returns_one_summary_per_baseline() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        &baseline_kinds,
    );

    let summary = summarize_multi_baseline_aggregates(&report);

    assert_eq!(summary.baseline_summaries.len(), baseline_kinds.len());
}

#[test]
fn multi_baseline_summary_preserves_baseline_order() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        &baseline_kinds,
    );

    let summary = summarize_multi_baseline_aggregates(&report);

    let names: Vec<&str> = summary
        .baseline_summaries
        .iter()
        .map(|baseline| baseline.baseline_name.as_str())
        .collect();

    assert_eq!(names, vec!["flat_scan", "kd_tree", "r_tree"]);
}

#[test]
fn multi_baseline_summary_uses_display_labels() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        &baseline_kinds,
    );

    let summary = summarize_multi_baseline_aggregates(&report);

    let labels: Vec<&str> = summary
        .baseline_summaries
        .iter()
        .map(|baseline| baseline.baseline_label.as_str())
        .collect();

    assert_eq!(labels, vec!["Flat Scan", "KD-Tree", "R-Tree"]);
}

#[test]
fn multi_baseline_summary_copies_aggregate_metrics() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let baseline_kinds = [BaselineKind::FlatScan];

    let report = run_multi_baseline_benchmark_suite(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        &baseline_kinds,
    );

    let summary = summarize_multi_baseline_aggregates(&report);

    let source_aggregate = &report.baseline_reports[0].report.aggregate;
    let baseline_summary = &summary.baseline_summaries[0];

    assert_eq!(
        baseline_summary.workload_count,
        source_aggregate.workload_count
    );
    assert_eq!(
        baseline_summary.total_baseline_evaluated_records,
        source_aggregate.total_baseline_evaluated_records
    );
    assert_eq!(
        baseline_summary.total_fse_reconstructed_records,
        source_aggregate.total_fse_reconstructed_records
    );
    assert_eq!(
        baseline_summary.weighted_timing_ratio,
        source_aggregate.weighted_timing_ratio
    );
}

#[test]
fn multi_baseline_summary_reports_highest_weighted_timing_ratio() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        &baseline_kinds,
    );

    let summary = summarize_multi_baseline_aggregates(&report);

    assert!(summary.highest_weighted_timing_ratio().is_some());
}
