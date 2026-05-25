//! Reusable owned-result buffer query tests.

use crate::benchmark::{clustered_points_2d, clustered_workload_cases, flat_scan};
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{
    QueryExecutionOptions, execute_query, execute_query_into, execute_query_into_with_options,
    execute_query_with_stats,
};
use crate::tests::support::sort_points;

fn build_small_index() -> (Vec<Vector>, crate::storage::FSEIndex) {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8).with_target_leaf_size(8));
    let index = builder.build(&points);

    (points, index)
}

#[test]
fn execute_query_into_matches_owned_result_query() {
    let (_points, index) = build_small_index();
    let workloads = clustered_workload_cases();

    for workload in workloads {
        let mut reusable_results = Vec::with_capacity(32);
        let mut into_results = Vec::new();

        let stats = execute_query_into(&index, &workload.query, &mut reusable_results);
        into_results.append(&mut reusable_results);

        let owned_report = execute_query_with_stats(&index, &workload.query);

        sort_points(&mut into_results);

        let mut expected_results = owned_report.results;
        sort_points(&mut expected_results);

        assert_eq!(
            into_results, expected_results,
            "execute_query_into results differed from owned-result query for workload `{}`",
            workload.name
        );

        assert_eq!(
            stats, owned_report.stats,
            "execute_query_into stats differed from owned-result query for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn execute_query_into_matches_flat_scan_for_small_benchmark_workloads() {
    let (points, index) = build_small_index();

    for workload in clustered_workload_cases() {
        let mut reusable_results = Vec::with_capacity(32);
        let _stats = execute_query_into(&index, &workload.query, &mut reusable_results);

        let mut expected_results = flat_scan(&points, &workload.query);

        sort_points(&mut reusable_results);
        sort_points(&mut expected_results);

        assert_eq!(
            reusable_results, expected_results,
            "execute_query_into results differed from flat scan for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn execute_query_into_reuses_existing_outer_result_capacity() {
    let (_points, index) = build_small_index();
    let workload = clustered_workload_cases()
        .into_iter()
        .find(|workload| workload.name == "cluster_boundary_range")
        .expect("small benchmark workloads should include cluster_boundary_range");

    let mut reusable_results = Vec::with_capacity(64);
    let original_capacity = reusable_results.capacity();

    let stats = execute_query_into(&index, &workload.query, &mut reusable_results);

    assert_eq!(reusable_results.len(), stats.matched_records);
    assert!(
        reusable_results.capacity() >= original_capacity,
        "execute_query_into should preserve reusable result buffer capacity"
    );
}

#[test]
fn execute_query_into_clears_previous_results_for_disjoint_query() {
    let (_points, index) = build_small_index();
    let mut reusable_results = execute_query(&index, &clustered_workload_cases()[0].query);

    assert!(
        !reusable_results.is_empty(),
        "test setup should start with a non-empty result buffer"
    );

    let disjoint_query =
        crate::query::QueryRegion::new(vec![-1000.0, -1000.0], vec![-900.0, -900.0]);
    let stats = execute_query_into(&index, &disjoint_query, &mut reusable_results);

    assert!(reusable_results.is_empty());
    assert_eq!(stats.matched_records, 0);
    assert_eq!(stats.reconstructed_records, 0);
}

#[test]
fn execute_query_into_parallel_options_preserve_exact_results() {
    let (_points, index) = build_small_index();
    let workload = clustered_workload_cases()
        .into_iter()
        .find(|workload| workload.name == "cluster_boundary_range")
        .expect("small benchmark workloads should include cluster_boundary_range");

    let mut reusable_results = Vec::with_capacity(32);

    let stats = execute_query_into_with_options(
        &index,
        &workload.query,
        QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(0),
        &mut reusable_results,
    );

    let owned_report = execute_query_with_stats(&index, &workload.query);

    let mut into_results = reusable_results;
    let mut expected_results = owned_report.results;

    sort_points(&mut into_results);
    sort_points(&mut expected_results);

    assert_eq!(into_results, expected_results);
    assert_eq!(stats, owned_report.stats);
}
