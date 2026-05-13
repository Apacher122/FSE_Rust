use crate::benchmark::{
    RepeatedTimingConfig, clustered_points_2d, clustered_workload_cases, run_benchmark_suite,
    run_benchmark_suite_repeated,
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
