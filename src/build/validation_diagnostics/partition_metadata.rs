//! Partition dimensional metadata validation diagnostics.

use crate::build::validation::partition_dimensional_metadata_is_valid;
use crate::storage::{FSEIndex, PartitionNode};

use super::types::{
    PartitionDimensionalMetadataDiagnostics, PartitionDimensionalMetadataViolation,
};

pub(super) fn partition_dimensional_metadata_diagnostics(
    index: &FSEIndex,
) -> PartitionDimensionalMetadataDiagnostics {
    PartitionDimensionalMetadataDiagnostics {
        index_dimensions: index.dimensions,
        index_dimensions_valid: index.dimensions > 0,
        root_valid: index.root < index.nodes.len(),
        violations: index
            .nodes
            .iter()
            .filter(|node| !partition_dimensional_metadata_is_valid(index.dimensions, node))
            .map(|node| partition_dimensional_metadata_violation(index.dimensions, node))
            .collect(),
    }
}

fn partition_dimensional_metadata_violation(
    index_dimensions: usize,
    node: &PartitionNode,
) -> PartitionDimensionalMetadataViolation {
    PartitionDimensionalMetadataViolation {
        node_id: node.id,
        index_dimensions,
        centroid_dimensions: node.centroid.len(),
        bounds_min_dimensions: node.bounds.min.len(),
        bounds_max_dimensions: node.bounds.max.len(),
        residual_dimensions: node.residuals.dimensions(),
        residual_dimension_lengths: node.residuals.dimension_lengths(),
        cardinality: node.cardinality,
        stored_cardinality: node.stored_cardinality(),
        is_leaf: node.is_leaf,
        centroid_finite: node.centroid.iter().all(|value| value.is_finite()),
        bounds_finite: node
            .bounds
            .min
            .iter()
            .chain(&node.bounds.max)
            .all(|value| value.is_finite()),
        bounds_ranges_valid: node.bounds.min.len() == node.bounds.max.len()
            && node
                .bounds
                .min
                .iter()
                .zip(&node.bounds.max)
                .all(|(minimum, maximum)| minimum <= maximum),
        residuals_finite: node
            .residuals
            .dimensions
            .iter()
            .flatten()
            .all(|value| value.is_finite()),
    }
}
