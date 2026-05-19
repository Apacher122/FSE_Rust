//! Residual reconstruction.

use crate::math::{Scalar, Vector};
use crate::storage::PartitionNode;

/// Validated reconstruction shape for a partition.
///
/// # Runtime Role
///
/// This small value lets hot execution paths validate partition reconstruction
/// shape once and then reuse the dimensionality and row count across every row
/// in the retained leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReconstructionShape {
    /// Number of coordinate dimensions reconstructed for each row.
    pub dimensions: usize,

    /// Number of residual rows available for reconstruction.
    pub cardinality: usize,
}

/// Validates the reconstruction shape of a partition.
///
/// # Runtime Role
///
/// Public reconstruction helpers must defend against malformed partition state.
/// Retained-leaf execution can call this once per leaf and then use the
/// prevalidated row reconstruction helpers inside the row loop.
///
/// # Panics
///
/// Panics when centroid and residual dimensionality are inconsistent or when
/// residual dimensions do not contain the same row count.
pub(crate) fn validate_partition_reconstruction_shape(node: &PartitionNode) -> ReconstructionShape {
    let dimensions = node.residuals.dimensions();
    let cardinality = node.residuals.cardinality();

    assert_eq!(
        node.centroid.len(),
        dimensions,
        "partition centroid and residual dimensionality must match"
    );

    for (dimension_index, residual_dimension) in node.residuals.dimensions.iter().enumerate() {
        assert_eq!(
            residual_dimension.len(),
            cardinality,
            "residual dimension {dimension_index} has {} rows but expected {cardinality}",
            residual_dimension.len()
        );
    }

    ReconstructionShape {
        dimensions,
        cardinality,
    }
}

/// Reconstructs one row from a partition into an existing coordinate buffer.
///
/// # Runtime Role
///
/// This function performs row-local deferred reconstruction without allocating
/// a new `Vector` for every candidate record. The caller owns the output buffer
/// and may reuse it across many rows.
///
/// # Formal Reference
///
/// This implements the reconstruction operator `Phi_k(Delta) = mu_k + Delta`
/// for one residual row.
///
/// # Panics
///
/// Panics when centroid and residual dimensionality are inconsistent, when the
/// residual row is out of range, or when residual dimensions have inconsistent
/// row counts.
pub fn reconstruct_row_into(node: &PartitionNode, row: usize, output: &mut Vec<Scalar>) {
    let shape = validate_partition_reconstruction_shape(node);

    assert!(
        row < shape.cardinality,
        "residual row index must be inside the partition cardinality"
    );

    reconstruct_row_into_prevalidated(node, row, shape.dimensions, output);
}

/// Reconstructs one row after the caller has already validated partition shape.
///
/// # Runtime Role
///
/// This is the retained-leaf hot-path variant. It avoids repeating the public
/// shape checks for every row in a leaf while preserving debug assertions that
/// catch misuse during development.
///
/// The output buffer is kept shaped to `dimensions` and overwritten in place
/// across rows. That avoids the clear-and-push loop in the partial retained-leaf
/// path while preserving the same public result semantics.
///
/// # Panics
///
/// In release builds, this function relies on the caller to pass a valid row
/// and a previously validated partition shape. Invalid inputs may still panic
/// through normal slice indexing.
#[inline]
pub(crate) fn reconstruct_row_into_prevalidated(
    node: &PartitionNode,
    row: usize,
    dimensions: usize,
    output: &mut Vec<Scalar>,
) {
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

    if output.len() != dimensions {
        output.resize(dimensions, 0.0);
    }

    // shape was already checked at the leaf boundary
    // keep the scratch vec sized and just overwrite it
    for (dimension, (centroid_value, residual_dimension)) in node
        .centroid
        .iter()
        .zip(&node.residuals.dimensions)
        .enumerate()
    {
        output[dimension] = *centroid_value + residual_dimension[row];
    }
}

/// Reconstructs one point after the caller has already validated partition shape.
///
/// # Runtime Role
///
/// Covered retained leaves can append every row directly to the result set.
/// This helper avoids routing each row through the public reconstruction wrapper.
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
/// This is a convenience wrapper around [`reconstruct_row_into`] for callers
/// that need an owned `Vector` for a single residual row.
///
/// # Formal Reference
///
/// This implements `Phi_k(Delta) = mu_k + Delta` for one encoded record.
///
/// # Panics
///
/// Panics under the same conditions as [`reconstruct_row_into`].
pub fn reconstruct_point(node: &PartitionNode, row: usize) -> Vector {
    let shape = validate_partition_reconstruction_shape(node);

    assert!(
        row < shape.cardinality,
        "residual row index must be inside the partition cardinality"
    );

    reconstruct_point_prevalidated(node, row, shape.dimensions)
}

/// Reconstructs all points stored in a partition.
///
/// # Runtime Role
///
/// Reconstruction performs Stage II of the FSE query pipeline. It materializes
/// absolute coordinates only after a partition has passed metadata pruning.
///
/// # Formal Reference
///
/// This implements the reconstruction operator `Phi_k(Delta) = mu_k + Delta`.
///
/// # Panics
///
/// Panics when centroid and residual dimensionality are inconsistent.
pub fn reconstruct_partition(node: &PartitionNode) -> Vec<Vector> {
    let shape = validate_partition_reconstruction_shape(node);
    let mut reconstructed = Vec::with_capacity(shape.cardinality);

    // keep this path for tests and callers that really need materialized rows
    for row in 0..shape.cardinality {
        reconstructed.push(reconstruct_point_prevalidated(node, row, shape.dimensions));
    }

    reconstructed
}
