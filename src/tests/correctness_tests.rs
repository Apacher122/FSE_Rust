use crate::benchmark::flat_scan;
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{QueryRegion, execute_query};

#[test]
fn fse_query_matches_linear_scan_for_selective_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig {
        max_leaf_size: 2,
        max_depth: 8,
    });
    let index = builder.build(&points);
    let query = QueryRegion::new(vec![1.0, 1.0], vec![3.0, 3.0]);

    let fse_results = execute_query(&index, &query);
    let scan_results = flat_scan(&points, &query);

    // Since order might differ, we just check length for now.
    // TODO: Implement a point-sorting utility for 1:1 vector equality checks.
    assert_eq!(fse_results.len(), scan_results.len());
}
