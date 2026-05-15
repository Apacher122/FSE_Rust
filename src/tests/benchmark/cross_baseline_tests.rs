use crate::benchmark::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, exact_range_baseline_kinds,
    large_clustered_points_2d, large_clustered_workload_cases,
};
use crate::math::Vector;
use crate::query::QueryRegion;
use crate::tests::support::assert_baselines_match_for_workloads;

#[test]
fn exact_baselines_match_for_small_clustered_workloads() {
    let points = clustered_points_2d();
    let workloads = clustered_workload_cases();

    assert_baselines_match_for_workloads(exact_range_baseline_kinds(), &points, &workloads);
}

#[test]
fn exact_baselines_match_for_large_clustered_workloads() {
    let points = large_clustered_points_2d();
    let workloads = large_clustered_workload_cases();

    // large workloads are still deterministic so this should stay stable
    assert_baselines_match_for_workloads(exact_range_baseline_kinds(), &points, &workloads);
}

#[test]
fn exact_baselines_match_for_boundary_and_duplicate_workloads() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
        Vector::new(vec![4.0, 1.0]),
        Vector::new(vec![-1.0, -1.0]),
    ];

    let workloads = vec![
        QueryWorkloadCase::new(
            "single_boundary_point",
            QueryRegion::new(vec![1.0, 1.0], vec![1.0, 1.0]),
        ),
        QueryWorkloadCase::new(
            "duplicate_point_range",
            QueryRegion::new(vec![0.5, 0.5], vec![1.5, 1.5]),
        ),
        QueryWorkloadCase::new(
            "non_diagonal_range",
            QueryRegion::new(vec![1.0, 0.0], vec![4.0, 2.0]),
        ),
        QueryWorkloadCase::new(
            "negative_boundary_range",
            QueryRegion::new(vec![-1.0, -1.0], vec![0.0, 0.0]),
        ),
        QueryWorkloadCase::new(
            "empty_far_range",
            QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]),
        ),
    ];

    // these are the annoying cases that usually break tree baselines first
    assert_baselines_match_for_workloads(exact_range_baseline_kinds(), &points, &workloads);
}
