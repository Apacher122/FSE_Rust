use std::time::Duration;

use crate::benchmark::{
    RepeatedTimingConfig, clustered_points_2d, compare_query_execution,
    compare_query_execution_repeated, duration_ratio, measure_elapsed, measure_repeated,
};
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
fn measure_repeated_reports_iteration_count() {
    let config = RepeatedTimingConfig::new(5);
    let report = measure_repeated(&config, || {
        let _ = 2 + 2;
    });

    assert_eq!(report.iterations, 5);
}

#[test]
fn measure_repeated_reports_average_duration() {
    let config = RepeatedTimingConfig::new(5);
    let report = measure_repeated(&config, || {
        let _ = 2 + 2;
    });

    assert!(report.average_elapsed >= Duration::ZERO);
}

#[test]
fn comparison_report_includes_timing_measurements() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![50.0, 50.0], vec![55.0, 55.0]);
    let report = compare_query_execution(&index, &points, &query);

    assert!(report.timing.baseline_elapsed >= Duration::ZERO);
    assert!(report.timing.fse_elapsed >= Duration::ZERO);
}

#[test]
fn repeated_comparison_report_uses_requested_iteration_count() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![50.0, 50.0], vec![55.0, 55.0]);
    let timing_config = RepeatedTimingConfig::new(3);

    let report = compare_query_execution_repeated(&index, &points, &query, &timing_config);

    assert_eq!(report.repeated_timing.baseline.iterations, 3);
    assert_eq!(report.repeated_timing.fse.iterations, 3);
}

#[test]
fn duration_ratio_divides_elapsed_values() {
    let numerator = Duration::from_secs(10);
    let denominator = Duration::from_secs(2);

    assert_eq!(duration_ratio(numerator, denominator), 5.0);
}

#[test]
fn duration_ratio_returns_zero_for_two_zero_durations() {
    assert_eq!(duration_ratio(Duration::ZERO, Duration::ZERO), 0.0);
}

#[test]
fn repeated_comparison_report_includes_timing_ratios() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![50.0, 50.0], vec![55.0, 55.0]);
    let timing_config = RepeatedTimingConfig::new(3);

    let report = compare_query_execution_repeated(&index, &points, &query, &timing_config);

    assert!(report.single_run_timing_ratio >= 0.0);
    assert!(report.average_timing_ratio >= 0.0);
}
