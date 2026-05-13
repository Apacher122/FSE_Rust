use crate::benchmark::{
    BaselineComparisonLabels, FlatScanBaseline, RangeQueryBaseline, execute_range_baseline,
    flat_scan,
};

use crate::math::Vector;
use crate::query::QueryRegion;

#[test]
fn flat_scan_baseline_reports_name() {
    let baseline = FlatScanBaseline;

    assert_eq!(baseline.name(), "flat_scan");
}

#[test]
fn flat_scan_baseline_matches_flat_scan_results() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let baseline = FlatScanBaseline;

    let report = baseline.execute(&points, &query);
    let direct = flat_scan(&points, &query);

    assert_eq!(report.results, direct);
}

#[test]
fn flat_scan_baseline_reports_common_stats() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let baseline = FlatScanBaseline;

    let report = execute_range_baseline(&baseline, &points, &query);

    assert_eq!(report.baseline_name, "flat_scan");
    assert_eq!(report.stats.evaluated_records, 4);
    assert_eq!(report.stats.matched_records, 2);
}

#[test]
fn flat_scan_baseline_reports_display_labels() {
    let baseline = FlatScanBaseline;
    let labels = baseline.labels();

    assert_eq!(labels.baseline_name, "flat_scan");
    assert_eq!(labels.baseline_label, "Flat Scan");
    assert_eq!(labels.fse_label, "FSE");
    assert_eq!(labels.comparison_label, "Flat Scan vs FSE");
}

#[test]
fn baseline_comparison_labels_fallback_replaces_underscores() {
    let labels = BaselineComparisonLabels::new("custom_baseline");

    assert_eq!(labels.baseline_name, "custom_baseline");
    assert_eq!(labels.baseline_label, "custom baseline");
    assert_eq!(labels.comparison_label, "custom baseline vs FSE");
}
