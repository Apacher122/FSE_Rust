use std::time::Duration;

use crate::benchmark::{clustered_points_2d, compare_query_execution, measure_elapsed};
use crate::build::{BuildConfig, FSEBuilder};
use crate::query::QueryRegion;

#[test]
fn measure_elapsed_returns_operation_result() {
    let (result, _elapsed) = measure_elapsed(|| 2 + 2);

    assert_eq!(result, 4);
}

#[test]
fn measure_elapsed_returns_duration() {
    let (_result, elapsed) = measure_elapsed(|| 2 + 2);

    assert!(elapsed >= Duration::ZERO);
}

#[test]
fn comparison_report_includes_timing_measurements() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![50.0, 50.0], vec![55.0, 55.0]);
    let report = compare_query_execution(&index, &points, &query);

    assert!(report.timing.flat_scan_elapsed >= Duration::ZERO);
    assert!(report.timing.fse_elapsed >= Duration::ZERO);
}
