//! Baseline footprint CSV fields.

use crate::benchmark::BaselineFootprintMetrics;

use super::document::format_ratio;

pub(super) fn baseline_footprint_header_fields() -> Vec<&'static str> {
    vec![
        "baseline_footprint_node_count",
        "baseline_footprint_leaf_count",
        "baseline_footprint_internal_node_count",
        "baseline_point_coordinate_scalar_count",
        "baseline_routing_metadata_scalar_count",
        "baseline_bounds_metadata_scalar_count",
        "baseline_structural_metadata_scalar_count",
        "baseline_total_scalar_count",
        "baseline_total_to_point_scalar_ratio",
        "baseline_structural_to_point_scalar_ratio",
    ]
}

pub(super) fn baseline_footprint_value_fields(metrics: &BaselineFootprintMetrics) -> Vec<String> {
    vec![
        metrics.node_count.to_string(),
        metrics.leaf_count.to_string(),
        metrics.internal_node_count.to_string(),
        metrics.point_coordinate_scalar_count.to_string(),
        metrics.routing_metadata_scalar_count.to_string(),
        metrics.bounds_metadata_scalar_count.to_string(),
        metrics.structural_metadata_scalar_count.to_string(),
        metrics.total_scalar_count.to_string(),
        format_ratio(metrics.total_to_point_scalar_ratio as f64),
        format_ratio(metrics.structural_to_point_scalar_ratio as f64),
    ]
}
