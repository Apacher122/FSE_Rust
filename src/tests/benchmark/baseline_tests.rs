use crate::benchmark::{
    BaselineComparisonLabels, BaselineKind, BaselineRegistry, FlatScanBaseline, KdTreeBaseline,
    RangeQueryBaseline, execute_range_baseline, flat_scan,
};

use crate::math::Vector;
use crate::query::QueryRegion;

#[test]
fn flat_scan_baseline_reports_name() {
    let baseline = FlatScanBaseline::new(&[]);

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
    let baseline = FlatScanBaseline::new(&points);

    let report = baseline.execute(&query);
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
    let baseline = FlatScanBaseline::new(&points);

    let report = execute_range_baseline(&baseline, &query);

    assert_eq!(report.baseline_name, "flat_scan");
    assert_eq!(report.stats.evaluated_records, 4);
    assert_eq!(report.stats.matched_records, 2);
}

#[test]
fn flat_scan_baseline_reports_display_labels() {
    let baseline = FlatScanBaseline::new(&[]);
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

#[test]
fn baseline_kind_reports_flat_scan_name() {
    assert_eq!(BaselineKind::FlatScan.name(), "flat_scan");
}

#[test]
fn baseline_registry_resolves_flat_scan() {
    let registry = BaselineRegistry::new();
    let baseline = registry.resolve(BaselineKind::FlatScan, &[]);

    assert_eq!(baseline.name(), "flat_scan");
}

#[test]
fn baseline_kind_reports_kd_tree_name() {
    assert_eq!(BaselineKind::KdTree.name(), "kd_tree");
}

#[test]
fn baseline_registry_resolves_kd_tree() {
    let registry = BaselineRegistry::new();
    let baseline = registry.resolve(BaselineKind::KdTree, &[]);

    assert_eq!(baseline.name(), "kd_tree");
}

#[test]
fn kd_tree_baseline_reports_display_labels() {
    let baseline = KdTreeBaseline::new(&[]);
    let labels = baseline.labels();

    assert_eq!(labels.baseline_name, "kd_tree");
    assert_eq!(labels.baseline_label, "KD-Tree");
    assert_eq!(labels.fse_label, "FSE");
    assert_eq!(labels.comparison_label, "KD-Tree vs FSE");
}
