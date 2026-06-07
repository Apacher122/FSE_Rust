//! Baseline footprint metrics.

use crate::math::Scalar;

use super::baseline::BaselineKind;

/// Logical footprint metrics for a benchmark baseline.
///
/// # Runtime Role
///
/// `BaselineFootprintMetrics` reports the logical scalar fields represented by
/// a baseline implementation. The fields are intended for benchmark reporting
/// and structural comparison with FSE index footprint metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaselineFootprintMetrics {
    /// Baseline implementation measured by this report.
    pub baseline_kind: BaselineKind,

    /// Dimensionality of the represented coordinate space.
    pub dimensions: usize,

    /// Number of records represented by the baseline.
    pub record_count: usize,

    /// Number of logical nodes represented by the baseline index.
    pub node_count: usize,

    /// Number of logical leaf nodes represented by the baseline index.
    pub leaf_count: usize,

    /// Number of logical internal nodes represented by the baseline index.
    pub internal_node_count: usize,

    /// Number of scalar coordinates stored for source points.
    pub point_coordinate_scalar_count: usize,

    /// Number of scalar fields used for routing metadata.
    pub routing_metadata_scalar_count: usize,

    /// Number of scalar fields used for bounding metadata.
    pub bounds_metadata_scalar_count: usize,

    /// Total scalar fields used for baseline metadata.
    pub structural_metadata_scalar_count: usize,

    /// Total scalar fields counted by this footprint report.
    pub total_scalar_count: usize,

    /// Total scalar count divided by point coordinate scalar count.
    pub total_to_point_scalar_ratio: Scalar,

    /// Structural metadata scalar count divided by point coordinate scalar count.
    pub structural_to_point_scalar_ratio: Scalar,
}

impl BaselineFootprintMetrics {
    /// Builds a flat scan footprint report.
    pub fn flat_scan(record_count: usize, dimensions: usize) -> Self {
        baseline_footprint_metrics(BaselineKind::FlatScan, record_count, dimensions, 0, 0, 0, 0)
    }

    /// Builds a KD-tree footprint report.
    pub fn kd_tree(
        record_count: usize,
        dimensions: usize,
        node_count: usize,
        leaf_count: usize,
    ) -> Self {
        baseline_footprint_metrics(
            BaselineKind::KdTree,
            record_count,
            dimensions,
            node_count,
            leaf_count,
            node_count,
            0,
        )
    }

    /// Builds an R-tree footprint report.
    pub fn r_tree(
        record_count: usize,
        dimensions: usize,
        node_count: usize,
        leaf_count: usize,
    ) -> Self {
        baseline_footprint_metrics(
            BaselineKind::RTree,
            record_count,
            dimensions,
            node_count,
            leaf_count,
            0,
            node_count * dimensions * 2,
        )
    }

    /// Returns true when the report represents no source records.
    pub fn is_empty(&self) -> bool {
        self.record_count == 0
    }
}

fn baseline_footprint_metrics(
    baseline_kind: BaselineKind,
    record_count: usize,
    dimensions: usize,
    node_count: usize,
    leaf_count: usize,
    routing_metadata_scalar_count: usize,
    bounds_metadata_scalar_count: usize,
) -> BaselineFootprintMetrics {
    let point_coordinate_scalar_count = record_count * dimensions;
    let structural_metadata_scalar_count =
        routing_metadata_scalar_count + bounds_metadata_scalar_count;
    let total_scalar_count = point_coordinate_scalar_count + structural_metadata_scalar_count;

    BaselineFootprintMetrics {
        baseline_kind,
        dimensions,
        record_count,
        node_count,
        leaf_count,
        internal_node_count: node_count.saturating_sub(leaf_count),
        point_coordinate_scalar_count,
        routing_metadata_scalar_count,
        bounds_metadata_scalar_count,
        structural_metadata_scalar_count,
        total_scalar_count,
        total_to_point_scalar_ratio: ratio(total_scalar_count, point_coordinate_scalar_count),
        structural_to_point_scalar_ratio: ratio(
            structural_metadata_scalar_count,
            point_coordinate_scalar_count,
        ),
    }
}

fn ratio(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        return 0.0;
    }

    numerator as Scalar / denominator as Scalar
}
