use crate::benchmark::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, flat_scan,
    large_clustered_points_2d, large_clustered_workload_cases,
};
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{QueryRegion, count_query_matches, execute_query};
use crate::tests::support::sort_points;

#[test]
fn fse_query_matches_linear_scan_for_selective_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![1.0, 1.0], vec![3.0, 3.0]);

    let mut fse_results = execute_query(&index, &query);
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut fse_results);
    sort_points(&mut scan_results);

    assert_eq!(fse_results, scan_results);
}

#[test]
fn fse_query_matches_linear_scan_for_empty_result() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);

    let fse_results = execute_query(&index, &query);
    let scan_results = flat_scan(&points, &query);

    assert_eq!(fse_results, scan_results);
}

#[test]
fn fse_query_matches_linear_scan_for_full_range_query() {
    let points = vec![
        Vector::new(vec![-5.0, -5.0]),
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 2.0]),
        Vector::new(vec![4.0, 8.0]),
        Vector::new(vec![10.0, 10.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![-10.0, -10.0], vec![20.0, 20.0]);

    let mut fse_results = execute_query(&index, &query);
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut fse_results);
    sort_points(&mut scan_results);

    assert_eq!(fse_results, scan_results);
}

#[test]
fn fse_query_matches_linear_scan_for_non_diagonal_region() {
    let points = vec![
        Vector::new(vec![0.0, 10.0]),
        Vector::new(vec![1.0, 9.0]),
        Vector::new(vec![2.0, 8.0]),
        Vector::new(vec![3.0, 7.0]),
        Vector::new(vec![4.0, 6.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![1.0, 7.0], vec![3.0, 9.0]);

    let mut fse_results = execute_query(&index, &query);
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut fse_results);
    sort_points(&mut scan_results);

    assert_eq!(fse_results, scan_results);
}

fn assert_fse_matches_flat_scan_for_benchmark_workloads(
    points: &[Vector],
    max_depth: usize,
    workloads: &[QueryWorkloadCase],
) {
    let builder = FSEBuilder::new(BuildConfig::new(8, max_depth).with_target_leaf_size(8));
    let index = builder.build(points);

    for workload in workloads {
        let mut fse_results = execute_query(&index, &workload.query);
        let mut expected_results = flat_scan(points, &workload.query);
        let count_only_matches = count_query_matches(&index, &workload.query);

        sort_points(&mut fse_results);
        sort_points(&mut expected_results);

        assert_eq!(
            fse_results, expected_results,
            "FSE owned-result query differed from flat scan for workload `{}`",
            workload.name
        );

        assert_eq!(
            count_only_matches,
            expected_results.len(),
            "FSE count-only query differed from flat scan cardinality for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn fse_query_matches_flat_scan_for_small_benchmark_workloads() {
    let points = clustered_points_2d();
    let workloads = clustered_workload_cases();

    assert_fse_matches_flat_scan_for_benchmark_workloads(&points, 8, &workloads);
}

#[test]
fn fse_query_matches_flat_scan_for_large_benchmark_workloads() {
    let points = large_clustered_points_2d();
    let workloads = large_clustered_workload_cases();

    assert_fse_matches_flat_scan_for_benchmark_workloads(&points, 16, &workloads);
}
