//! Serial query execution determinism tests.

use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{
    QueryExecutionOptions, QueryExecutionReport, QueryRegion, execute_query_with_options,
    execute_query_with_stats_and_options,
};
use crate::storage::FSEIndex;

fn deterministic_test_points() -> Vec<Vector> {
    vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
        Vector::new(vec![4.0, 4.0]),
        Vector::new(vec![5.0, 5.0]),
        Vector::new(vec![6.0, 6.0]),
        Vector::new(vec![7.0, 7.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ]
}

fn build_deterministic_test_index() -> FSEIndex {
    let points = deterministic_test_points();
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    builder.build(&points)
}

fn deterministic_test_query() -> QueryRegion {
    QueryRegion::new(vec![2.0, 2.0], vec![7.0, 7.0])
}

fn execute_serial_report(index: &FSEIndex, query: &QueryRegion) -> QueryExecutionReport {
    execute_query_with_stats_and_options(index, query, QueryExecutionOptions::serial())
}

#[test]
fn serial_query_execution_returns_identical_results_across_repeated_runs() {
    let index = build_deterministic_test_index();
    let query = deterministic_test_query();

    let expected = execute_query_with_options(&index, &query, QueryExecutionOptions::serial());

    // samee query should not wiggle around between runs
    for _ in 0..10 {
        let actual = execute_query_with_options(&index, &query, QueryExecutionOptions::serial());

        assert_eq!(actual, expected);
    }
}

#[test]
fn serial_query_execution_returns_identical_stats_across_repeated_runs() {
    let index = build_deterministic_test_index();
    let query = deterministic_test_query();

    let expected = execute_serial_report(&index, &query);

    // this is the baseline before rayon can make order annoying
    for _ in 0..10 {
        let actual = execute_serial_report(&index, &query);

        assert_eq!(actual.stats, expected.stats);
    }
}

#[test]
fn serial_query_execution_returns_identical_reports_across_repeated_runs() {
    let index = build_deterministic_test_index();
    let query = deterministic_test_query();

    let expected = execute_serial_report(&index, &query);

    // full report check keeps result order and stats tied together
    for _ in 0..10 {
        let actual = execute_serial_report(&index, &query);

        assert_eq!(actual, expected);
    }
}

#[test]
fn serial_query_execution_preserves_expected_result_order() {
    let index = build_deterministic_test_index();
    let query = deterministic_test_query();

    let report = execute_serial_report(&index, &query);

    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![3.0, 3.0]),
            Vector::new(vec![4.0, 4.0]),
            Vector::new(vec![5.0, 5.0]),
            Vector::new(vec![6.0, 6.0]),
            Vector::new(vec![7.0, 7.0]),
        ]
    );
}

#[test]
fn serial_query_execution_reports_expected_work_for_deterministic_index() {
    let index = build_deterministic_test_index();
    let query = deterministic_test_query();

    let report = execute_serial_report(&index, &query);

    assert_eq!(report.stats.total_records, 10);
    assert_eq!(report.stats.matched_records, 6);

    assert!(
        report.stats.visited_nodes > 0,
        "serial query should visit at least the root node"
    );

    assert!(
        report.stats.total_leaves > 1,
        "determinism fixture should build multiple leaves"
    );

    assert!(
        report.stats.retained_leaves > 0,
        "query should retain at least one leaf"
    );

    assert!(
        report.stats.reconstructed_records >= report.stats.matched_records,
        "candidate rows must include every matched row"
    );
}
