use crate::benchmark::{
    BaselineKind, compare_query_execution_with_options,
    run_multi_baseline_benchmark_suite_with_options,
};
use crate::query::execution::DEFAULT_PARALLEL_MIN_RETAINED_LEAVES;
use crate::query::{QueryExecutionOptions, execute_query_with_stats_and_options};
use crate::tests::support::small_benchmark_fixture;

fn parallel_threshold_options(threshold: usize) -> QueryExecutionOptions {
    QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(threshold)
}

fn threshold_cases() -> Vec<usize> {
    vec![0, 1, DEFAULT_PARALLEL_MIN_RETAINED_LEAVES, usize::MAX]
}

#[test]
fn parallel_thresholds_preserve_query_reports_for_standard_workloads() {
    let fixture = small_benchmark_fixture();

    assert!(
        !fixture.workloads.is_empty(),
        "benchmark fixture should include workload cases"
    );

    for workload in &fixture.workloads {
        let serial_report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        for threshold in threshold_cases() {
            let threshold_report = execute_query_with_stats_and_options(
                &fixture.index,
                &workload.query,
                parallel_threshold_options(threshold),
            );

            assert_eq!(
                threshold_report, serial_report,
                "parallel threshold `{}` changed query report for workload `{}`",
                threshold, workload.name
            );
        }
    }
}

#[test]
fn parallel_thresholds_preserve_query_stats_for_standard_workloads() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let serial_report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        for threshold in threshold_cases() {
            let threshold_report = execute_query_with_stats_and_options(
                &fixture.index,
                &workload.query,
                parallel_threshold_options(threshold),
            );

            assert_eq!(
                threshold_report.stats, serial_report.stats,
                "parallel threshold `{}` changed query stats for workload `{}`",
                threshold, workload.name
            );
        }
    }
}

#[test]
fn parallel_thresholds_preserve_benchmark_comparison_accounting() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let serial_comparison = compare_query_execution_with_options(
            &fixture.index,
            &fixture.points,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        for threshold in threshold_cases() {
            let threshold_comparison = compare_query_execution_with_options(
                &fixture.index,
                &fixture.points,
                &workload.query,
                parallel_threshold_options(threshold),
            );

            assert_eq!(
                threshold_comparison.fse_stats, serial_comparison.fse_stats,
                "parallel threshold `{}` changed FSE stats for workload `{}`",
                threshold, workload.name
            );

            assert_eq!(
                threshold_comparison.baseline_stats, serial_comparison.baseline_stats,
                "parallel threshold `{}` changed baseline stats for workload `{}`",
                threshold, workload.name
            );

            assert_eq!(
                threshold_comparison.avoided_reconstructions,
                serial_comparison.avoided_reconstructions,
                "parallel threshold `{}` changed avoided reconstruction count for workload `{}`",
                threshold,
                workload.name
            );

            assert_eq!(
                threshold_comparison.reconstruction_avoidance_ratio,
                serial_comparison.reconstruction_avoidance_ratio,
                "parallel threshold `{}` changed reconstruction avoidance ratio for workload `{}`",
                threshold,
                workload.name
            );

            assert_eq!(
                threshold_comparison.candidate_ratio, serial_comparison.candidate_ratio,
                "parallel threshold `{}` changed candidate ratio for workload `{}`",
                threshold, workload.name
            );

            assert_eq!(
                threshold_comparison.retained_leaf_ratio, serial_comparison.retained_leaf_ratio,
                "parallel threshold `{}` changed retained leaf ratio for workload `{}`",
                threshold, workload.name
            );
        }
    }
}

#[test]
fn parallel_thresholds_preserve_multi_baseline_report_shape_and_stats() {
    let fixture = small_benchmark_fixture();
    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let serial_report = run_multi_baseline_benchmark_suite_with_options(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        &baseline_kinds,
        QueryExecutionOptions::serial(),
    );

    for threshold in threshold_cases() {
        let threshold_report = run_multi_baseline_benchmark_suite_with_options(
            &fixture.index,
            &fixture.points,
            &fixture.workloads,
            &fixture.timing_config,
            &fixture.registry,
            &baseline_kinds,
            parallel_threshold_options(threshold),
        );

        assert_eq!(
            threshold_report.baseline_reports.len(),
            serial_report.baseline_reports.len(),
            "parallel threshold `{}` changed baseline report count",
            threshold
        );

        for (threshold_baseline, serial_baseline) in threshold_report
            .baseline_reports
            .iter()
            .zip(&serial_report.baseline_reports)
        {
            assert_eq!(
                threshold_baseline.baseline_kind, serial_baseline.baseline_kind,
                "parallel threshold `{}` changed baseline kind ordering",
                threshold
            );

            assert_eq!(
                threshold_baseline.baseline_name, serial_baseline.baseline_name,
                "parallel threshold `{}` changed baseline name ordering",
                threshold
            );

            assert_eq!(
                threshold_baseline.report.comparisons.len(),
                serial_baseline.report.comparisons.len(),
                "parallel threshold `{}` changed workload comparison count for baseline `{}`",
                threshold,
                serial_baseline.baseline_name
            );

            for (threshold_comparison, serial_comparison) in threshold_baseline
                .report
                .comparisons
                .iter()
                .zip(&serial_baseline.report.comparisons)
            {
                assert_eq!(
                    threshold_comparison.workload_name, serial_comparison.workload_name,
                    "parallel threshold `{}` changed workload ordering for baseline `{}`",
                    threshold, serial_baseline.baseline_name
                );

                assert_eq!(
                    threshold_comparison.comparison.fse_stats,
                    serial_comparison.comparison.fse_stats,
                    "parallel threshold `{}` changed FSE stats for workload `{}` and baseline `{}`",
                    threshold,
                    serial_comparison.workload_name,
                    serial_baseline.baseline_name
                );

                assert_eq!(
                    threshold_comparison.comparison.baseline_stats,
                    serial_comparison.comparison.baseline_stats,
                    "parallel threshold `{}` changed baseline stats for workload `{}` and baseline `{}`",
                    threshold,
                    serial_comparison.workload_name,
                    serial_baseline.baseline_name
                );

                assert_eq!(
                    threshold_comparison.comparison.candidate_ratio,
                    serial_comparison.comparison.candidate_ratio,
                    "parallel threshold `{}` changed candidate ratio for workload `{}` and baseline `{}`",
                    threshold,
                    serial_comparison.workload_name,
                    serial_baseline.baseline_name
                );

                assert_eq!(
                    threshold_comparison.comparison.retained_leaf_ratio,
                    serial_comparison.comparison.retained_leaf_ratio,
                    "parallel threshold `{}` changed retained leaf ratio for workload `{}` and baseline `{}`",
                    threshold,
                    serial_comparison.workload_name,
                    serial_baseline.baseline_name
                );
            }
        }
    }
}

#[test]
fn parallel_thresholds_preserve_result_lengths_for_standard_workloads() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let serial_report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        for threshold in threshold_cases() {
            let threshold_report = execute_query_with_stats_and_options(
                &fixture.index,
                &workload.query,
                parallel_threshold_options(threshold),
            );

            assert_eq!(
                threshold_report.results.len(),
                serial_report.results.len(),
                "parallel threshold `{}` changed result length for workload `{}`",
                threshold,
                workload.name
            );

            assert_eq!(
                threshold_report.stats.matched_records,
                threshold_report.results.len(),
                "parallel threshold `{}` produced inconsistent matched count for workload `{}`",
                threshold,
                workload.name
            );

            assert!(
                threshold_report.stats.reconstructed_records
                    >= threshold_report.stats.matched_records,
                "parallel threshold `{}` reconstructed fewer rows than it matched for workload `{}`",
                threshold,
                workload.name
            );
        }
    }
}
