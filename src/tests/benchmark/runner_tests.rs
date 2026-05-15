use crate::benchmark::{
    BaselineKind, run_benchmark_suite, run_benchmark_suite_repeated,
    run_benchmark_suite_with_registry, run_multi_baseline_benchmark_suite,
};

use crate::tests::support::small_benchmark_fixture;

#[test]
fn benchmark_runner_returns_one_comparison_per_workload() {
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite(&fixture.index, &fixture.points, &fixture.workloads);

    assert_eq!(report.comparisons.len(), fixture.workloads.len());
}

#[test]
fn benchmark_runner_returns_one_pruning_report_per_workload() {
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite(&fixture.index, &fixture.points, &fixture.workloads);

    assert_eq!(report.pruning_reports.len(), fixture.workloads.len());
}

#[test]
fn benchmark_runner_preserves_workload_names_across_reports() {
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite(&fixture.index, &fixture.points, &fixture.workloads);

    for ((workload, comparison), pruning_report) in fixture
        .workloads
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
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite(&fixture.index, &fixture.points, &fixture.workloads);

    assert_eq!(report.aggregate.workload_count, fixture.workloads.len());
    assert_eq!(
        report.aggregate.total_baseline_evaluated_records,
        fixture.points.len() * fixture.workloads.len()
    );
}

#[test]
fn benchmark_runner_repeated_uses_requested_timing_iterations() {
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite_repeated(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
    );

    for comparison in report.comparisons {
        assert_eq!(
            comparison.comparison.repeated_timing.baseline.iterations,
            fixture.timing_config.iterations
        );
        assert_eq!(
            comparison.comparison.repeated_timing.fse.iterations,
            fixture.timing_config.iterations
        );
    }
}

#[test]
fn benchmark_runner_with_registry_uses_selected_baseline() {
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite_with_registry(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        BaselineKind::FlatScan,
    );

    for comparison in report.comparisons {
        assert_eq!(comparison.comparison.baseline_name, "flat_scan");
    }
}

#[test]
fn benchmark_runner_with_registry_can_use_kd_tree_baseline() {
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite_with_registry(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        BaselineKind::KdTree,
    );

    for comparison in report.comparisons {
        assert_eq!(comparison.comparison.baseline_name, "kd_tree");
        assert_eq!(comparison.comparison.labels.baseline_label, "KD-Tree");
    }
}

#[test]
fn benchmark_runner_with_registry_can_use_r_tree_baseline() {
    let fixture = small_benchmark_fixture();

    let report = run_benchmark_suite_with_registry(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        BaselineKind::RTree,
    );

    for comparison in report.comparisons {
        assert_eq!(comparison.comparison.baseline_name, "r_tree");
        assert_eq!(comparison.comparison.labels.baseline_label, "R-Tree");
    }
}

#[test]
fn multi_baseline_runner_returns_one_report_per_baseline() {
    let fixture = small_benchmark_fixture();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        &baseline_kinds,
    );

    assert_eq!(report.baseline_reports.len(), baseline_kinds.len());
}

#[test]
fn multi_baseline_runner_preserves_baseline_order() {
    let fixture = small_benchmark_fixture();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
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
    let fixture = small_benchmark_fixture();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        &baseline_kinds,
    );

    for baseline_report in report.baseline_reports {
        assert_eq!(
            baseline_report.report.comparisons.len(),
            fixture.workloads.len()
        );
        assert_eq!(
            baseline_report.report.pruning_reports.len(),
            fixture.workloads.len()
        );
        assert_eq!(
            baseline_report.report.aggregate.workload_count,
            fixture.workloads.len()
        );
    }
}

#[test]
fn multi_baseline_runner_comparisons_use_matching_baseline_names() {
    let fixture = small_benchmark_fixture();

    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
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
