//! Partition reconstruction helpers.

use crate::math::Vector;
use crate::storage::PartitionNode;

use super::point::reconstruct_point_prevalidated;
use super::shape::validate_partition_reconstruction_shape;

/// Reconstructs all points stored in a partition.
///
/// # Runtime Role
///
/// Reconstruction performs Stage II of the FSE query pipeline. It materializes
/// absolute coordinates only after a partition has passed metadata pruning.
///
/// # Formal Reference
///
/// This implements the reconstruction operator $\Phi_k(\Delta) = \mu_k + \Delta$.
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
