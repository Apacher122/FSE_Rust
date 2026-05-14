use crate::benchmark::{KdTreeBaseline, RangeQueryBaseline, flat_scan};
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

#[test]
fn kd_tree_baseline_matches_flat_scan_for_selective_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let kd_tree = KdTreeBaseline::new(&points);

    let mut kd_results = kd_tree.execute(&query).results;
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut kd_results);
    sort_points(&mut scan_results);

    assert_eq!(kd_results, scan_results);
}

#[test]
fn kd_tree_baseline_matches_flat_scan_for_empty_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);
    let kd_tree = KdTreeBaseline::new(&points);

    let mut kd_results = kd_tree.execute(&query).results;
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut kd_results);
    sort_points(&mut scan_results);

    assert_eq!(kd_results, scan_results);
}

#[test]
fn kd_tree_baseline_matches_flat_scan_for_full_range_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let query = QueryRegion::new(vec![-1.0, -1.0], vec![10.0, 10.0]);
    let kd_tree = KdTreeBaseline::new(&points);

    let mut kd_results = kd_tree.execute(&query).results;
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut kd_results);
    sort_points(&mut scan_results);

    assert_eq!(kd_results, scan_results);
}

#[test]
fn kd_tree_baseline_reports_common_stats() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let kd_tree = KdTreeBaseline::new(&points);
    let report = kd_tree.execute(&query);

    assert_eq!(report.baseline_name, "kd_tree");
    assert_eq!(report.stats.matched_records, 2);
    assert!(report.stats.evaluated_records <= points.len());
}
