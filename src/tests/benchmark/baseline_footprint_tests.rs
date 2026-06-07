use crate::benchmark::{
    BaselineFootprintMetrics, BaselineKind, FlatScanBaseline, KdTreeBaseline, RTreeBaseline,
    RangeQueryBaseline,
};
use crate::math::Vector;

#[test]
fn flat_scan_footprint_reports_coordinate_storage_only() {
    let points = two_dimensional_points();
    let baseline = FlatScanBaseline::new(&points);

    let metrics = baseline.footprint_metrics();

    assert_eq!(metrics.baseline_kind, BaselineKind::FlatScan);
    assert_eq!(metrics.dimensions, 2);
    assert_eq!(metrics.record_count, 5);
    assert_eq!(metrics.node_count, 0);
    assert_eq!(metrics.leaf_count, 0);
    assert_eq!(metrics.internal_node_count, 0);
    assert_eq!(metrics.point_coordinate_scalar_count, 10);
    assert_eq!(metrics.routing_metadata_scalar_count, 0);
    assert_eq!(metrics.bounds_metadata_scalar_count, 0);
    assert_eq!(metrics.structural_metadata_scalar_count, 0);
    assert_eq!(metrics.total_scalar_count, 10);
    assert_eq!(metrics.total_to_point_scalar_ratio, 1.0);
    assert_eq!(metrics.structural_to_point_scalar_ratio, 0.0);
}

#[test]
fn kd_tree_footprint_reports_point_storage_and_split_metadata() {
    let points = two_dimensional_points();
    let baseline = KdTreeBaseline::new(&points);

    let metrics = baseline.footprint_metrics();

    assert_eq!(metrics.baseline_kind, BaselineKind::KdTree);
    assert_eq!(metrics.dimensions, 2);
    assert_eq!(metrics.record_count, 5);
    assert_eq!(metrics.node_count, 5);
    assert_eq!(metrics.leaf_count, 2);
    assert_eq!(metrics.internal_node_count, 3);
    assert_eq!(metrics.point_coordinate_scalar_count, 10);
    assert_eq!(metrics.routing_metadata_scalar_count, 5);
    assert_eq!(metrics.bounds_metadata_scalar_count, 0);
    assert_eq!(metrics.structural_metadata_scalar_count, 5);
    assert_eq!(metrics.total_scalar_count, 15);
    assert_eq!(metrics.total_to_point_scalar_ratio, 1.5);
    assert_eq!(metrics.structural_to_point_scalar_ratio, 0.5);
}

#[test]
fn r_tree_footprint_reports_point_storage_and_bounds_metadata() {
    let points = two_dimensional_points();
    let baseline = RTreeBaseline::new(&points);

    let metrics = baseline.footprint_metrics();

    assert_eq!(metrics.baseline_kind, BaselineKind::RTree);
    assert_eq!(metrics.dimensions, 2);
    assert_eq!(metrics.record_count, 5);
    assert_eq!(metrics.node_count, 1);
    assert_eq!(metrics.leaf_count, 1);
    assert_eq!(metrics.internal_node_count, 0);
    assert_eq!(metrics.point_coordinate_scalar_count, 10);
    assert_eq!(metrics.routing_metadata_scalar_count, 0);
    assert_eq!(metrics.bounds_metadata_scalar_count, 4);
    assert_eq!(metrics.structural_metadata_scalar_count, 4);
    assert_eq!(metrics.total_scalar_count, 14);
    assert_scalar_eq(metrics.total_to_point_scalar_ratio, 1.4);
    assert_scalar_eq(metrics.structural_to_point_scalar_ratio, 0.4);
}

#[test]
fn baseline_footprint_metric_constructors_handle_empty_baselines() {
    let metrics = BaselineFootprintMetrics::kd_tree(0, 0, 0, 0);

    assert!(metrics.is_empty());
    assert_eq!(metrics.total_scalar_count, 0);
    assert_eq!(metrics.total_to_point_scalar_ratio, 0.0);
    assert_eq!(metrics.structural_to_point_scalar_ratio, 0.0);
}

#[test]
fn range_query_baseline_trait_exposes_footprint_metrics() {
    let points = two_dimensional_points();
    let baseline: Box<dyn RangeQueryBaseline> = Box::new(KdTreeBaseline::new(&points));

    let metrics = baseline.footprint_metrics();

    assert_eq!(metrics.baseline_kind, BaselineKind::KdTree);
    assert_eq!(metrics.record_count, 5);
    assert_eq!(metrics.total_scalar_count, 15);
}

fn two_dimensional_points() -> Vec<Vector> {
    vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ]
}

fn assert_scalar_eq(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= f32::EPSILON,
        "expected {left} to equal {right}"
    );
}
