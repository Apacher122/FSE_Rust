use crate::benchmark::{
    KdTreeBaseline, RangeQueryBaseline, clustered_points_2d, clustered_workload_cases, flat_scan,
    large_clustered_points_2d, large_clustered_workload_cases,
};
use crate::math::Vector;
use crate::query::QueryRegion;

fn sort_points(points: &mut [Vector]) {
    points.sort_by(|left, right| {
        for (left_value, right_value) in left.values.iter().zip(&right.values) {
            match left_value.partial_cmp(right_value) {
                Some(std::cmp::Ordering::Equal) => continue,
                Some(ordering) => return ordering,
                None => return std::cmp::Ordering::Equal,
            }
        }

        left.values.len().cmp(&right.values.len())
    });
}

fn assert_kd_tree_matches_flat_scan(points: &[Vector], query: &QueryRegion) {
    let kd_tree = KdTreeBaseline::new(points);

    let kd_report = kd_tree.execute(query);
    let scan_results = flat_scan(points, query);

    let mut kd_results = kd_report.results;
    let mut expected_results = scan_results;

    sort_points(&mut kd_results);
    sort_points(&mut expected_results);

    assert_eq!(kd_results, expected_results);
    assert_eq!(kd_report.stats.matched_records, expected_results.len());
    assert!(kd_report.stats.evaluated_records <= points.len());
}

#[test]
fn kd_tree_matches_flat_scan_for_small_clustered_workloads() {
    let points = clustered_points_2d();
    let workloads = clustered_workload_cases();

    for workload in workloads {
        assert_kd_tree_matches_flat_scan(&points, &workload.query);
    }
}

#[test]
fn kd_tree_matches_flat_scan_for_large_clustered_workloads() {
    let points = large_clustered_points_2d();
    let workloads = large_clustered_workload_cases();

    for workload in workloads {
        assert_kd_tree_matches_flat_scan(&points, &workload.query);
    }
}

#[test]
fn kd_tree_matches_flat_scan_for_boundary_touching_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);

    assert_kd_tree_matches_flat_scan(&points, &query);
}

#[test]
fn kd_tree_matches_flat_scan_for_non_diagonal_query_region() {
    let points = vec![
        Vector::new(vec![0.0, 10.0]),
        Vector::new(vec![1.0, 9.0]),
        Vector::new(vec![2.0, 8.0]),
        Vector::new(vec![3.0, 7.0]),
        Vector::new(vec![4.0, 6.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 7.0], vec![3.0, 9.0]);

    assert_kd_tree_matches_flat_scan(&points, &query);
}
