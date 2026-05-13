use crate::benchmark::{
    aggregate_workload_metrics, clustered_points_2d, clustered_workload_cases, duration_ratio,
    summarize_workload_comparisons,
};
use crate::build::{BuildConfig, FSEBuilder};
use std::time::Duration;

#[test]
fn workload_summary_returns_one_summary_per_workload() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);

    assert_eq!(summaries.len(), workloads.len());
}

#[test]
fn workload_summary_preserves_workload_names() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);

    let summary_names: Vec<&str> = summaries
        .iter()
        .map(|summary| summary.workload_name.as_str())
        .collect();

    let workload_names: Vec<&str> = workloads
        .iter()
        .map(|workload| workload.name.as_str())
        .collect();

    assert_eq!(summary_names, workload_names);
}

#[test]
fn workload_summary_reports_flat_scan_record_count() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);

    for summary in summaries {
        assert_eq!(
            summary.comparison.baseline_stats.evaluated_records,
            points.len()
        );
    }
}

#[test]
fn aggregate_workload_metrics_counts_workloads() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    assert_eq!(aggregate.workload_count, workloads.len());
}

#[test]
fn aggregate_workload_metrics_totals_scan_evaluations() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    assert_eq!(
        aggregate.total_scan_evaluated_records,
        points.len() * workloads.len()
    );
}

#[test]
fn aggregate_workload_metrics_handles_empty_summary_list() {
    let aggregate = aggregate_workload_metrics(&[]);

    assert_eq!(aggregate.workload_count, 0);
    assert_eq!(aggregate.total_scan_evaluated_records, 0);
    assert_eq!(aggregate.total_fse_visited_nodes, 0);
    assert_eq!(aggregate.total_fse_retained_leaves, 0);
    assert_eq!(aggregate.total_fse_reconstructed_records, 0);
    assert_eq!(aggregate.total_fse_matched_records, 0);
    assert_eq!(aggregate.total_avoided_reconstructions, 0);
    assert_eq!(aggregate.average_reconstruction_avoidance_ratio, 0.0);
    assert_eq!(aggregate.average_candidate_ratio, 0.0);
    assert_eq!(aggregate.average_retained_leaf_ratio, 0.0);
    assert_eq!(aggregate.weighted_reconstruction_avoidance_ratio, 0.0);
    assert_eq!(aggregate.weighted_candidate_ratio, 0.0);
    assert_eq!(aggregate.total_flat_scan_average_elapsed, Duration::ZERO);
    assert_eq!(aggregate.total_fse_average_elapsed, Duration::ZERO);
    assert_eq!(aggregate.mean_flat_scan_average_elapsed, Duration::ZERO);
    assert_eq!(aggregate.mean_flat_scan_average_elapsed, Duration::ZERO);
    assert_eq!(aggregate.mean_timing_ratio, 0.0);
    assert_eq!(aggregate.weighted_timing_ratio, 0.0);
}

#[test]
fn aggregate_workload_metrics_reports_average_candidate_ratio() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    assert!(aggregate.average_candidate_ratio >= 0.0);
    assert!(aggregate.average_candidate_ratio <= 1.0);
}

#[test]
fn aggregate_workload_metrics_reports_average_retained_leaf_ratio() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    assert!(aggregate.average_retained_leaf_ratio >= 0.0);
    assert!(aggregate.average_retained_leaf_ratio <= 1.0);
}

#[test]
fn aggregate_workload_metrics_reports_weighted_reconstruction_avoidance_ratio() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    let expected = aggregate.total_avoided_reconstructions as f32
        / aggregate.total_scan_evaluated_records as f32;

    assert_eq!(aggregate.weighted_reconstruction_avoidance_ratio, expected);
}

#[test]
fn aggregate_workload_metrics_reports_weighted_candidate_ratio() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    let expected = aggregate.total_fse_reconstructed_records as f32
        / aggregate.total_scan_evaluated_records as f32;

    assert_eq!(aggregate.weighted_candidate_ratio, expected);
}

#[test]
fn aggregate_workload_metrics_derives_mean_timing_from_totals() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    let expected_flat_scan_mean = Duration::from_secs_f64(
        aggregate.total_flat_scan_average_elapsed.as_secs_f64() / aggregate.workload_count as f64,
    );

    let expected_fse_mean = Duration::from_secs_f64(
        aggregate.total_fse_average_elapsed.as_secs_f64() / aggregate.workload_count as f64,
    );

    assert_eq!(
        aggregate.mean_flat_scan_average_elapsed,
        expected_flat_scan_mean
    );
    assert_eq!(aggregate.mean_fse_average_elapsed, expected_fse_mean);
}

#[test]
fn aggregate_workload_metrics_derives_weighted_timing_ratio_from_totals() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    let expected = duration_ratio(
        aggregate.total_flat_scan_average_elapsed,
        aggregate.total_fse_average_elapsed,
    );

    assert_eq!(aggregate.weighted_timing_ratio, expected);
}

#[test]
fn aggregate_workload_metrics_derives_mean_timing_ratio_from_workloads() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);
    let workloads = clustered_workload_cases();

    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

    let expected = summaries
        .iter()
        .map(|summary| summary.comparison.average_timing_ratio)
        .sum::<f64>()
        / summaries.len() as f64;

    assert_eq!(aggregate.mean_timing_ratio, expected);
}
