//! Logical index footprint metrics.

use std::mem::size_of;

use crate::math::Scalar;
use crate::storage::FSEIndex;

use super::types::{
    IndexFootprintByteEstimates, IndexFootprintComparisonMetrics, IndexFootprintMetrics,
};

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

/// Computes footprint comparison metrics for an FSE index.
///
/// # Runtime Role
///
/// The comparison uses the encoded coordinate scalar count as the baseline.
/// The result identifies scalar overhead and the share of the index footprint
/// assigned to geometric metadata.
pub fn index_footprint_comparison_metrics(index: &FSEIndex) -> IndexFootprintComparisonMetrics {
    let footprint = index_footprint_metrics(index);

    footprint_comparison_metrics(&footprint)
}

/// Computes byte estimates for an FSE index footprint.
///
/// # Runtime Role
///
/// The estimates scale logical scalar counts by `size_of::<Scalar>()`. This keeps
/// byte accounting tied to the scalar representation used by the current build.
pub fn index_footprint_byte_estimates(index: &FSEIndex) -> IndexFootprintByteEstimates {
    let footprint = index_footprint_metrics(index);

    footprint_byte_estimates(&footprint)
}

/// Computes footprint comparison metrics from an existing footprint report.
pub fn footprint_comparison_metrics(
    footprint: &IndexFootprintMetrics,
) -> IndexFootprintComparisonMetrics {
    let encoded_baseline_scalar_count = footprint.encoded_coordinate_scalar_count;
    let index_scalar_count = footprint.total_index_scalar_count;

    IndexFootprintComparisonMetrics {
        encoded_baseline_scalar_count,
        index_scalar_count,
        scalar_delta_from_baseline: scalar_delta(index_scalar_count, encoded_baseline_scalar_count),
        residual_scalar_count: footprint.residual_scalar_count,
        structural_metadata_scalar_count: footprint.structural_metadata_scalar_count,
        index_to_encoded_baseline_scalar_ratio: ratio(
            index_scalar_count,
            encoded_baseline_scalar_count,
        ),
        residual_to_encoded_baseline_scalar_ratio: ratio(
            footprint.residual_scalar_count,
            encoded_baseline_scalar_count,
        ),
        structural_metadata_to_encoded_baseline_scalar_ratio: ratio(
            footprint.structural_metadata_scalar_count,
            encoded_baseline_scalar_count,
        ),
        structural_metadata_share_of_index: ratio(
            footprint.structural_metadata_scalar_count,
            index_scalar_count,
        ),
        index_exceeds_encoded_baseline: index_scalar_count > encoded_baseline_scalar_count,
        structural_metadata_dominates_residuals: footprint.structural_metadata_scalar_count
            > footprint.residual_scalar_count,
    }
}

/// Computes byte estimates from an existing footprint report.
pub fn footprint_byte_estimates(footprint: &IndexFootprintMetrics) -> IndexFootprintByteEstimates {
    let scalar_size_bytes = size_of::<Scalar>();
    let encoded_coordinate_bytes =
        scalar_byte_count(footprint.encoded_coordinate_scalar_count, scalar_size_bytes);
    let residual_bytes = scalar_byte_count(footprint.residual_scalar_count, scalar_size_bytes);
    let centroid_bytes = scalar_byte_count(footprint.centroid_scalar_count, scalar_size_bytes);
    let bounds_bytes = scalar_byte_count(footprint.bounds_scalar_count, scalar_size_bytes);
    let structural_metadata_bytes = centroid_bytes + bounds_bytes;
    let total_index_bytes = residual_bytes + structural_metadata_bytes;

    IndexFootprintByteEstimates {
        scalar_size_bytes,
        encoded_coordinate_bytes,
        residual_bytes,
        centroid_bytes,
        bounds_bytes,
        structural_metadata_bytes,
        total_index_bytes,
    }
}

fn ratio(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        return 0.0;
    }

    numerator as Scalar / denominator as Scalar
}

fn scalar_delta(value: usize, baseline: usize) -> i128 {
    value as i128 - baseline as i128
}

fn scalar_byte_count(scalar_count: usize, scalar_size_bytes: usize) -> usize {
    scalar_count * scalar_size_bytes
}
