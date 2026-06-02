//! Partition dimensional metadata validation.

use crate::math::BoundingBox;
use crate::storage::{FSEIndex, PartitionNode};

/// Validates dimensional metadata for every partition node in an index.
///
/// # Runtime Role
///
/// This check verifies the partition shape invariants required before
/// traversal and reconstruction can rely on stored geometry.
///
/// # Validation Rule
///
/// For every partition `P_k`, centroid dimensionality, bounded-support
/// dimensionality, residual dimensionality, and stored residual row shape must
/// agree with the index dimensionality.
pub fn validate_partition_dimensional_metadata(index: &FSEIndex) -> bool {
    if index.nodes.is_empty() || index.root >= index.nodes.len() || index.dimensions == 0 {
        return false;
    }

    index
        .nodes
        .iter()
        .all(|node| partition_dimensional_metadata_is_valid(index.dimensions, node))
}

pub(crate) fn partition_dimensional_metadata_is_valid(
    index_dimensions: usize,
    node: &PartitionNode,
) -> bool {
    let dimensions = node.dimensions();

    if dimensions == 0 || dimensions != index_dimensions {
        return false;
    }

    if !node.centroid.iter().all(|value| value.is_finite()) {
        return false;
    }

    if node.bounds.min.len() != dimensions || node.bounds.max.len() != dimensions {
        return false;
    }

    if !bounds_ranges_are_valid(&node.bounds) {
        return false;
    }

    if node.residuals.dimensions() != dimensions || !node.residuals.has_consistent_shape() {
        return false;
    }

    if !node
        .residuals
        .dimensions
        .iter()
        .flatten()
        .all(|value| value.is_finite())
    {
        return false;
    }

    if node.is_leaf {
        node.stored_cardinality() == node.cardinality
    } else {
        node.stored_cardinality() <= node.cardinality
    }
}

pub(crate) fn bounds_ranges_are_valid(bounds: &BoundingBox) -> bool {
    bounds_shape_is_valid(bounds)
        && bounds
            .min
            .iter()
            .zip(&bounds.max)
            .all(|(minimum, maximum)| {
                minimum.is_finite() && maximum.is_finite() && minimum <= maximum
            })
}

pub(crate) fn bounds_shape_is_valid(bounds: &BoundingBox) -> bool {
    bounds.min.len() == bounds.max.len()
}
