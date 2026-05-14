use crate::benchmark::{
    RTreeBaseline, clustered_points_2d, clustered_workload_cases, large_clustered_points_2d,
    large_clustered_workload_cases,
};
use crate::math::Vector;
use crate::query::QueryRegion;

use crate::tests::support::assert_baseline_matches_flat_scan;

#[test]
fn r_tree_matches_flat_scan_for_small_clustered_workloads() {
    let points = clustered_points_2d();
    let workloads = clustered_workload_cases();

    for workload in workloads {
        assert_baseline_matches_flat_scan(RTreeBaseline::new(&points), &points, &workload.query);
    }
}

#[test]
fn r_tree_matches_flat_scan_for_large_clustered_workloads() {
    let points = large_clustered_points_2d();
    let workloads = large_clustered_workload_cases();

    for workload in workloads {
        assert_baseline_matches_flat_scan(RTreeBaseline::new(&points), &points, &workload.query);
    }
}

#[test]
fn r_tree_matches_flat_scan_for_boundary_touching_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);

    assert_baseline_matches_flat_scan(RTreeBaseline::new(&points), &points, &query);
}

#[test]
fn r_tree_matches_flat_scan_for_non_diagonal_query_region() {
    let points = vec![
        Vector::new(vec![0.0, 10.0]),
        Vector::new(vec![1.0, 9.0]),
        Vector::new(vec![2.0, 8.0]),
        Vector::new(vec![3.0, 7.0]),
        Vector::new(vec![4.0, 6.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 7.0], vec![3.0, 9.0]);

    assert_baseline_matches_flat_scan(RTreeBaseline::new(&points), &points, &query);
}
