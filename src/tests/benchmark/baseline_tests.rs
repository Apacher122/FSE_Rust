use crate::benchmark::{
    BaselineComparisonLabels, BaselineKind, BaselineRegistry, BenchmarkBaselineSet,
    EXACT_RANGE_BASELINE_KINDS, FlatScanBaseline, KdTreeBaseline, RTreeBaseline,
    RangeQueryBaseline, baseline_kind_name_list, baseline_kind_names, exact_range_baseline_kinds,
    exact_range_baseline_vec, execute_range_baseline, flat_scan, has_multiple_baselines,
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

#[test]
fn baseline_kind_reports_r_tree_name() {
    assert_eq!(BaselineKind::RTree.name(), "r_tree");
}

#[test]
fn baseline_registry_resolves_r_tree() {
    let registry = BaselineRegistry::new();
    let baseline = registry.resolve(BaselineKind::RTree, &[]);

    assert_eq!(baseline.name(), "r_tree");
}

#[test]
fn r_tree_baseline_reports_display_labels() {
    let baseline = RTreeBaseline::new(&[]);
    let labels = baseline.labels();

    assert_eq!(labels.baseline_name, "r_tree");
    assert_eq!(labels.baseline_label, "R-Tree");
    assert_eq!(labels.fse_label, "FSE");
    assert_eq!(labels.comparison_label, "R-Tree vs FSE");
}

#[test]
fn exact_range_baseline_kinds_returns_canonical_baseline_list() {
    assert_eq!(
        exact_range_baseline_kinds(),
        &[
            BaselineKind::FlatScan,
            BaselineKind::KdTree,
            BaselineKind::RTree,
        ]
    );
}

#[test]
fn exact_range_baseline_kinds_uses_shared_constant() {
    assert_eq!(exact_range_baseline_kinds(), &EXACT_RANGE_BASELINE_KINDS);
}

#[test]
fn exact_range_baseline_vec_returns_owned_baseline_list() {
    let baselines = exact_range_baseline_vec();

    assert_eq!(
        baselines,
        vec![
            BaselineKind::FlatScan,
            BaselineKind::KdTree,
            BaselineKind::RTree,
        ]
    );
}

#[test]
fn baseline_kind_names_returns_stable_names() {
    let names = baseline_kind_names(&[BaselineKind::FlatScan, BaselineKind::RTree]);

    assert_eq!(names, vec!["flat_scan", "r_tree"]);
}

#[test]
fn baseline_kind_name_list_joins_stable_names() {
    let names = baseline_kind_name_list(&[BaselineKind::FlatScan, BaselineKind::KdTree]);

    assert_eq!(names, "flat_scan, kd_tree");
}

#[test]
fn has_multiple_baselines_detects_multi_baseline_runs() {
    assert!(!has_multiple_baselines(&[BaselineKind::FlatScan]));
    assert!(has_multiple_baselines(&[
        BaselineKind::FlatScan,
        BaselineKind::KdTree
    ]));
}

#[test]
fn single_baseline_set_returns_one_selected_kind() {
    let baseline_set = BenchmarkBaselineSet::Single(BaselineKind::KdTree);

    assert_eq!(baseline_set.name(), "kd_tree");
    assert_eq!(baseline_set.selected_kinds(), vec![BaselineKind::KdTree]);
    assert_eq!(baseline_set.selected_names(), vec!["kd_tree"]);
    assert_eq!(baseline_set.selected_name_list(), "kd_tree");
    assert!(!baseline_set.is_multi_baseline());
}

#[test]
fn all_exact_baseline_set_returns_canonical_exact_baselines() {
    let baseline_set = BenchmarkBaselineSet::AllExact;

    assert_eq!(baseline_set.name(), "all_exact");
    assert_eq!(baseline_set.selected_kinds(), exact_range_baseline_vec());
    assert_eq!(
        baseline_set.selected_name_list(),
        "flat_scan, kd_tree, r_tree"
    );
    assert!(baseline_set.is_multi_baseline());
}
