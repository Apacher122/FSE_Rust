use crate::benchmark::{RTreeBaseline, RangeQueryBaseline, flat_scan};
use crate::math::Vector;
use crate::query::QueryRegion;

use crate::tests::support::sort_points;

#[test]
fn r_tree_baseline_matches_flat_scan_for_selective_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let r_tree = RTreeBaseline::new(&points);

    let mut r_tree_results = r_tree.execute(&query).results;
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut r_tree_results);
    sort_points(&mut scan_results);

    assert_eq!(r_tree_results, scan_results);
}

#[test]
fn r_tree_baseline_matches_flat_scan_for_empty_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);
    let r_tree = RTreeBaseline::new(&points);

    let mut r_tree_results = r_tree.execute(&query).results;
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut r_tree_results);
    sort_points(&mut scan_results);

    assert_eq!(r_tree_results, scan_results);
}

#[test]
fn r_tree_baseline_matches_flat_scan_for_full_range_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let query = QueryRegion::new(vec![-1.0, -1.0], vec![10.0, 10.0]);
    let r_tree = RTreeBaseline::new(&points);

    let mut r_tree_results = r_tree.execute(&query).results;
    let mut scan_results = flat_scan(&points, &query);

    sort_points(&mut r_tree_results);
    sort_points(&mut scan_results);

    assert_eq!(r_tree_results, scan_results);
}

#[test]
fn r_tree_baseline_reports_common_stats() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let r_tree = RTreeBaseline::new(&points);
    let report = r_tree.execute(&query);

    assert_eq!(report.baseline_name, "r_tree");
    assert_eq!(report.stats.matched_records, 2);
    assert!(report.stats.evaluated_records <= points.len());
}
