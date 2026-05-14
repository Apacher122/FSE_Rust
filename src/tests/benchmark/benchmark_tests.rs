use crate::benchmark::{flat_scan, flat_scan_with_stats};
use crate::math::Vector;
use crate::query::QueryRegion;

#[test]
fn flat_scan_returns_points_inside_query_region() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let results = flat_scan(&points, &query);

    assert_eq!(
        results,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0]),]
    );
}

#[test]
fn flat_scan_with_stats_reports_evaluated_and_matched_records() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let report = flat_scan_with_stats(&points, &query);

    assert_eq!(report.stats.evaluated_records, 4);
    assert_eq!(report.stats.matched_records, 2);
    assert_eq!(
        report.results,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0]),]
    );
}

#[test]
fn flat_scan_returns_empty_result_when_no_points_match() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);
    let report = flat_scan_with_stats(&points, &query);

    assert!(report.results.is_empty());
    assert_eq!(report.stats.evaluated_records, 2);
    assert_eq!(report.stats.matched_records, 0);
}
