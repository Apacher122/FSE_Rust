//! Serial and parallel query execution checks over benchmark workloads.

use crate::benchmark::flat_scan;
use crate::query::{
    QueryExecutionOptions, execute_query_with_options, execute_query_with_stats_and_options,
};
use crate::tests::support::{large_benchmark_fixture, small_benchmark_fixture, sort_points};

#[test]
fn parallel_query_reports_match_serial_reports_for_benchmark_workloads() {
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

        let parallel_report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::parallel(),
        );

        assert_eq!(
            parallel_report, serial_report,
            "parallel report differed from serial report for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn parallel_query_results_match_serial_results_for_benchmark_workloads() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let serial_results = execute_query_with_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        let parallel_results = execute_query_with_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::parallel(),
        );

        assert_eq!(
            parallel_results, serial_results,
            "parallel results differed from serial results for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn parallel_query_stats_match_serial_stats_for_benchmark_workloads() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let serial_report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        let parallel_report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::parallel(),
        );

        assert_eq!(
            parallel_report.stats, serial_report.stats,
            "parallel stats differed from serial stats for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn parallel_query_execution_is_deterministic_for_benchmark_workloads() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let expected = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::parallel(),
        );

        // rayon should not make the query output wobble around
        for _ in 0..10 {
            let actual = execute_query_with_stats_and_options(
                &fixture.index,
                &workload.query,
                QueryExecutionOptions::parallel(),
            );

            assert_eq!(
                actual, expected,
                "parallel execution was not deterministic for workload `{}`",
                workload.name
            );
        }
    }
}

#[test]
fn serial_and_parallel_query_results_match_flat_scan_for_benchmark_workloads() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let mut expected = flat_scan(&fixture.points, &workload.query);

        let mut serial_results = execute_query_with_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        let mut parallel_results = execute_query_with_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::parallel(),
        );

        sort_points(&mut expected);
        sort_points(&mut serial_results);
        sort_points(&mut parallel_results);

        assert_eq!(
            serial_results, expected,
            "serial FSE results differed from flat scan for workload `{}`",
            workload.name
        );

        assert_eq!(
            parallel_results, expected,
            "parallel FSE results differed from flat scan for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn parallel_query_reports_preserve_candidate_accounting_for_benchmark_workloads() {
    let fixture = small_benchmark_fixture();

    for workload in &fixture.workloads {
        let report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::parallel(),
        );

        assert!(
            report.stats.reconstructed_records >= report.stats.matched_records,
            "parallel reconstructed records must include matched records for workload `{}`",
            workload.name
        );

        assert_eq!(
            report.stats.matched_records,
            report.results.len(),
            "parallel matched record count must equal result length for workload `{}`",
            workload.name
        );

        assert!(
            report.stats.candidate_ratio >= 0.0 && report.stats.candidate_ratio <= 1.0,
            "parallel candidate ratio must be bounded for workload `{}`",
            workload.name
        );

        assert!(
            report.stats.retained_leaf_ratio >= 0.0 && report.stats.retained_leaf_ratio <= 1.0,
            "parallel retained leaf ratio must be bounded for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn forced_parallel_query_reports_match_serial_reports_for_large_benchmark_workloads() {
    let fixture = large_benchmark_fixture();
    let parallel_options = QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(0);

    assert!(
        !fixture.workloads.is_empty(),
        "large benchmark fixture should include workload cases"
    );

    for workload in &fixture.workloads {
        let serial_report = execute_query_with_stats_and_options(
            &fixture.index,
            &workload.query,
            QueryExecutionOptions::serial(),
        );

        let parallel_report =
            execute_query_with_stats_and_options(&fixture.index, &workload.query, parallel_options);

        assert_eq!(
            parallel_report, serial_report,
            "forced parallel report differed from serial report for large workload `{}`",
            workload.name
        );
    }
}

#[test]
fn forced_parallel_query_results_match_flat_scan_for_large_benchmark_workloads() {
    let fixture = large_benchmark_fixture();
    let parallel_options = QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(0);

    for workload in &fixture.workloads {
        let mut expected = flat_scan(&fixture.points, &workload.query);

        let mut parallel_results =
            execute_query_with_options(&fixture.index, &workload.query, parallel_options);

        sort_points(&mut expected);
        sort_points(&mut parallel_results);

        assert_eq!(
            parallel_results, expected,
            "forced parallel FSE results differed from flat scan for large workload `{}`",
            workload.name
        );
    }
}
