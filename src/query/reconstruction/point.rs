//! Point reconstruction helpers.

use crate::math::Vector;
use crate::storage::PartitionNode;

use super::shape::validate_partition_reconstruction_shape;

/// Reconstructs one point after the caller has already validated partition shape.
///
/// # Runtime Role
///
/// Covered retained leaves can append every row directly to the result set.
/// This helper avoids routing each row through the public reconstruction wrapper.
///
/// The 1D and 2D branches avoid the generic dimension loop for the current small
/// benchmark path. Higher-dimensional datasets keep the general reconstruction
/// loop so the implementation remains dimension-agnostic.
#[inline]
pub(crate) fn reconstruct_point_prevalidated(
    node: &PartitionNode,
    row: usize,
    dimensions: usize,
) -> Vector {
    debug_assert_eq!(
        node.centroid.len(),
        dimensions,
        "prevalidated centroid dimensionality should match"
    );
    debug_assert_eq!(
        node.residuals.dimensions.len(),
        dimensions,
        "prevalidated residual dimensionality should match"
    );
    debug_assert!(
        row < node.residuals.cardinality(),
        "prevalidated residual row should be inside cardinality"
    );

    match dimensions {
        1 => reconstruct_point_1d_prevalidated(node, row),
        2 => reconstruct_point_2d_prevalidated(node, row),
        _ => reconstruct_point_generic_prevalidated(node, row, dimensions),
    }
}

#[inline]
fn reconstruct_point_1d_prevalidated(node: &PartitionNode, row: usize) -> Vector {
    Vector::new(vec![node.centroid[0] + node.residuals.dimensions[0][row]])
}

#[inline]
fn reconstruct_point_2d_prevalidated(node: &PartitionNode, row: usize) -> Vector {
    Vector::new(vec![
        node.centroid[0] + node.residuals.dimensions[0][row],
        node.centroid[1] + node.residuals.dimensions[1][row],
    ])
}

#[inline]
fn reconstruct_point_generic_prevalidated(
    node: &PartitionNode,
    row: usize,
    dimensions: usize,
) -> Vector {
    let mut values = Vec::with_capacity(dimensions);

    // covered rows still need owned result vectors
    for (centroid_value, residual_dimension) in node.centroid.iter().zip(&node.residuals.dimensions)
    {
        values.push(*centroid_value + residual_dimension[row]);
    }

    Vector::new(values)
}

/// Reconstructs one point stored in a partition.
///
/// # Runtime Role
///
/// This is a convenience wrapper around [`super::reconstruct_row_into`] for callers
/// that need an owned [`Vector`] for a single residual row.
///
/// # Formal Reference
///
/// This implements $\Phi_k(\Delta) = \mu_k + \Delta$ for one encoded record.
///
/// # Panics
///
/// Panics when centroid and residual dimensionality are inconsistent, when the
/// residual row is out of range, or when residual dimensions have inconsistent
/// row counts.
pub fn reconstruct_point(node: &PartitionNode, row: usize) -> Vector {
    let shape = validate_partition_reconstruction_shape(node);

    assert!(
        row < shape.cardinality,
        "residual row index must be inside the partition cardinality"
    );

    reconstruct_point_prevalidated(node, row, shape.dimensions)
}
