use crate::benchmark::compare_query_execution;
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::QueryRegion;

#[test]
fn comparison_report_counts_avoided_reconstructions() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![0.0, 0.0], vec![1.0, 1.0]);
    let report = compare_query_execution(&index, &points, &query);

    assert_eq!(report.baseline_name, "flat_scan");
    assert_eq!(report.labels.baseline_label, "Flat Scan");
    assert_eq!(report.labels.fse_label, "FSE");
    assert_eq!(report.labels.comparison_label, "Flat Scan vs FSE");
    assert_eq!(report.baseline_stats.evaluated_records, 4);
    assert_eq!(report.baseline_stats.matched_records, 2);

    assert_eq!(report.fse_stats.retained_leaves, 1);
    assert_eq!(report.fse_stats.reconstructed_records, 2);
    assert_eq!(report.fse_stats.matched_records, 2);

    assert_eq!(report.avoided_reconstructions, 2);
    assert_eq!(report.reconstruction_avoidance_ratio, 0.5);
    assert_eq!(report.candidate_ratio, 0.5);
    assert_eq!(report.retained_leaf_ratio, 0.5);
}

#[test]
fn comparison_report_handles_full_range_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![-1.0, -1.0], vec![10.0, 10.0]);
    let report = compare_query_execution(&index, &points, &query);

    assert_eq!(report.baseline_stats.evaluated_records, 4);
    assert_eq!(report.baseline_stats.matched_records, 4);

    assert_eq!(report.fse_stats.retained_leaves, 2);
    assert_eq!(report.fse_stats.reconstructed_records, 4);
    assert_eq!(report.fse_stats.matched_records, 4);

    assert_eq!(report.avoided_reconstructions, 0);
    assert_eq!(report.reconstruction_avoidance_ratio, 0.0);
    assert_eq!(report.candidate_ratio, 1.0);
    assert_eq!(report.retained_leaf_ratio, 1.0);
}

#[test]
fn comparison_report_handles_empty_result_query() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![20.0, 20.0], vec![30.0, 30.0]);
    let report = compare_query_execution(&index, &points, &query);

    assert_eq!(report.baseline_stats.evaluated_records, 4);
    assert_eq!(report.baseline_stats.matched_records, 0);

    assert_eq!(report.fse_stats.retained_leaves, 0);
    assert_eq!(report.fse_stats.reconstructed_records, 0);
    assert_eq!(report.fse_stats.matched_records, 0);

    assert_eq!(report.avoided_reconstructions, 4);
    assert_eq!(report.reconstruction_avoidance_ratio, 1.0);
    assert_eq!(report.candidate_ratio, 0.0);
    assert_eq!(report.retained_leaf_ratio, 0.0);
}
