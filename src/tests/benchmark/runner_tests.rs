use crate::benchmark::{
    BaselineKind, BaselineRegistry, RepeatedTimingConfig, clustered_points_2d,
    clustered_workload_cases, run_benchmark_suite, run_benchmark_suite_repeated,
    run_benchmark_suite_with_registry, run_multi_baseline_benchmark_suite,
};
use crate::build::{BuildConfig, FSEBuilder};

#[test]
fn benchmark_runner_returns_one_comparison_per_workload() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let report = run_benchmark_suite(&index, &points, &workloads);

    assert_eq!(report.comparisons.len(), workloads.len());
}

#[test]
fn benchmark_runner_returns_one_pruning_report_per_workload() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let report = run_benchmark_suite(&index, &points, &workloads);

    assert_eq!(report.pruning_reports.len(), workloads.len());
}

#[test]
fn benchmark_runner_preserves_workload_names_across_reports() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let report = run_benchmark_suite(&index, &points, &workloads);

    for ((workload, comparison), pruning_report) in workloads
        .iter()
        .zip(&report.comparisons)
        .zip(&report.pruning_reports)
    {
        assert_eq!(comparison.workload_name, workload.name);
        assert_eq!(pruning_report.workload_name, workload.name);
    }
}

#[test]
fn benchmark_runner_populates_aggregate_metrics() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let report = run_benchmark_suite(&index, &points, &workloads);

    assert_eq!(report.aggregate.workload_count, workloads.len());
    assert_eq!(
        report.aggregate.total_baseline_evaluated_records,
        points.len() * workloads.len()
    );
}

#[test]
fn benchmark_runner_repeated_uses_requested_timing_iterations() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);

    let report = run_benchmark_suite_repeated(&index, &points, &workloads, &timing_config);

    for comparison in report.comparisons {
        assert_eq!(comparison.comparison.repeated_timing.baseline.iterations, 3);
        assert_eq!(comparison.comparison.repeated_timing.fse.iterations, 3);
    }
}

#[test]
fn benchmark_runner_with_registry_uses_selected_baseline() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let report = run_benchmark_suite_with_registry(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        BaselineKind::FlatScan,
    );

    for comparison in report.comparisons {
        assert_eq!(comparison.comparison.baseline_name, "flat_scan");
    }
}

#[test]
fn benchmark_runner_with_registry_can_use_kd_tree_baseline() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let report = run_benchmark_suite_with_registry(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        BaselineKind::KdTree,
    );

    for comparison in report.comparisons {
        assert_eq!(comparison.comparison.baseline_name, "kd_tree");
        assert_eq!(comparison.comparison.labels.baseline_label, "KD-Tree");
    }
}

#[test]
fn benchmark_runner_with_registry_can_use_r_tree_baseline() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    let report = run_benchmark_suite_with_registry(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        BaselineKind::RTree,
    );

    for comparison in report.comparisons {
        assert_eq!(comparison.comparison.baseline_name, "r_tree");
        assert_eq!(comparison.comparison.labels.baseline_label, "R-Tree");
    }
}

#[test]
fn multi_baseline_runner_returns_one_report_per_baseline() {
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

    assert_eq!(report.baseline_reports.len(), baseline_kinds.len());
}

#[test]
fn multi_baseline_runner_preserves_baseline_order() {
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

    let names: Vec<&str> = report
        .baseline_reports
        .iter()
        .map(|baseline_report| baseline_report.baseline_name.as_str())
        .collect();

    assert_eq!(names, vec!["flat_scan", "kd_tree", "r_tree"]);
}

#[test]
fn multi_baseline_runner_populates_each_baseline_report() {
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

    for baseline_report in report.baseline_reports {
        assert_eq!(baseline_report.report.comparisons.len(), workloads.len());
        assert_eq!(
            baseline_report.report.pruning_reports.len(),
            workloads.len()
        );
        assert_eq!(
            baseline_report.report.aggregate.workload_count,
            workloads.len()
        );
    }
}

#[test]
fn multi_baseline_runner_comparisons_use_matching_baseline_names() {
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

    for baseline_report in report.baseline_reports {
        for summary in baseline_report.report.comparisons {
            assert_eq!(
                summary.comparison.baseline_name,
                baseline_report.baseline_name
            );
        }
    }
}
