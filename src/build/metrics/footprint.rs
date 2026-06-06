//! Logical index footprint metrics.

use crate::math::Scalar;
use crate::storage::FSEIndex;

use super::types::IndexFootprintMetrics;

/// Computes logical scalar footprint metrics for an FSE index.
///
/// # Runtime Role
///
/// These metrics count the coordinate-like scalar values stored by the index.
/// They provide a deterministic accounting layer for comparing encoded input
/// size, residual storage, and geometric query metadata.
pub fn index_footprint_metrics(index: &FSEIndex) -> IndexFootprintMetrics {
    let record_count = index.root_node().cardinality;
    let encoded_coordinate_scalar_count = record_count * index.dimensions;
    let centroid_scalar_count = index.nodes.len() * index.dimensions;
    let bounds_scalar_count = index.nodes.len() * index.dimensions * 2;
    let residual_scalar_count = index
        .nodes
        .iter()
        .map(|node| {
            node.residuals
                .dimensions
                .iter()
                .map(Vec::len)
                .sum::<usize>()
        })
        .sum();
    let structural_metadata_scalar_count = centroid_scalar_count + bounds_scalar_count;
    let total_index_scalar_count = structural_metadata_scalar_count + residual_scalar_count;

    IndexFootprintMetrics {
        dimensions: index.dimensions,
        record_count,
        node_count: index.nodes.len(),
        leaf_count: index.leaf_count(),
        encoded_coordinate_scalar_count,
        residual_scalar_count,
        centroid_scalar_count,
        bounds_scalar_count,
        structural_metadata_scalar_count,
        total_index_scalar_count,
        residual_to_encoded_scalar_ratio: ratio(
            residual_scalar_count,
            encoded_coordinate_scalar_count,
        ),
        structural_to_encoded_scalar_ratio: ratio(
            structural_metadata_scalar_count,
            encoded_coordinate_scalar_count,
        ),
        index_to_encoded_scalar_ratio: ratio(
            total_index_scalar_count,
            encoded_coordinate_scalar_count,
        ),
    }
}

fn ratio(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        return 0.0;
    }

    numerator as Scalar / denominator as Scalar
}
