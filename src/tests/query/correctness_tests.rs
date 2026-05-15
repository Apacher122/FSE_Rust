use crate::benchmark::flat_scan;
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{QueryRegion, execute_query};
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
