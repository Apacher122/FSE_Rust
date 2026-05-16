//! Residual reconstruction.

use crate::math::{Scalar, Vector};
use crate::storage::PartitionNode;

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
    let dimensions = node.residuals.dimensions();
    let cardinality = node.residuals.cardinality();

    assert_eq!(
        node.centroid.len(),
        dimensions,
        "partition centroid and residual dimensionality must match"
    );

    assert!(
        row < cardinality,
        "residual row index must be inside the partition cardinality"
    );

    output.clear();

    if output.capacity() < dimensions {
        output.reserve(dimensions);
    }

    // dont build the whole leaf just to read one row
    for dimension in 0..dimensions {
        let residual_dimension = &node.residuals.dimensions[dimension];

        assert_eq!(
            residual_dimension.len(),
            cardinality,
            "all residual dimensions must have the same cardinality"
        );

        output.push(node.centroid[dimension] + residual_dimension[row]);
    }
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
    let mut values = Vec::with_capacity(node.residuals.dimensions());

    reconstruct_row_into(node, row, &mut values);

    Vector::new(values)
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
    let count = node.residuals.cardinality();
    let mut reconstructed = Vec::with_capacity(count);

    // keep this path for tests and callers that really need materialized rows
    for row in 0..count {
        reconstructed.push(reconstruct_point(node, row));
    }

    reconstructed
}
