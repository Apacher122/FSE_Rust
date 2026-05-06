//! Residual reconstruction.

use crate::math::Vector;
use crate::storage::PartitionNode;

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
pub fn reconstruct_partition(node: &PartitionNode) -> Vec<Vector> {
    let dimensions = node.residuals.dimensions();

    assert_eq!(
        node.centroid.len(),
        dimensions,
        "partition centroid and residual dimensionality must match"
    );

    let count = node.residuals.cardinality();
    let mut reconstructed = Vec::with_capacity(count);

    for row in 0..count {
        let mut values = Vec::with_capacity(dimensions);

        // this inner loop is a bottleneck.
        // it's a good candidate for AVX2/SIMD vectorization since it's just adding the centroid to the residual array.
        for dimension in 0..dimensions {
            values.push(node.centroid[dimension] + node.residuals.dimensions[dimension][row]);
        }

        reconstructed.push(Vector::new(values));
    }

    reconstructed
}
