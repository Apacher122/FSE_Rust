//! Structural metrics for partition quality.

use crate::math::Scalar;
use crate::storage::{FSEIndex, PartitionNode};

/// Computes the structural density of a partition.
///
/// # Runtime Role
///
/// Structural density measures record concentration relative to admissible
/// bounding volume.
///
/// # Formal Reference
///
/// This implements `delta(P_k) = |D_k| / Vol(B_k)`.
///
/// # Notes
///
/// If the bounding volume is zero, this returns positive infinity for non-empty
/// partitions and zero for empty partitions.
pub fn partition_density(node: &PartitionNode) -> Scalar {
    let volume = node.bounds.volume();
    if volume == 0.0 {
        return if node.cardinality == 0 {
            0.0
        } else {
            Scalar::INFINITY
        };
    }
    node.cardinality as Scalar / volume
}

/// Computes aggregate structural density across leaf nodes.
///
/// # Runtime Role
///
/// Global density estimates geometric efficiency over the physical query leaves
/// that store reconstructable residual rows.
///
/// # Formal Reference
///
/// This implements `delta(F) = N / sum Vol(B_k)` over leaf partitions.
pub fn index_density(index: &FSEIndex) -> Scalar {
    let leaves: Vec<&PartitionNode> = index.nodes.iter().filter(|node| node.is_leaf).collect();
    let total_cardinality: usize = leaves.iter().map(|node| node.cardinality).sum();
    let total_volume: Scalar = leaves.iter().map(|node| node.bounds.volume()).sum();

    if total_volume == 0.0 {
        return if total_cardinality == 0 {
            0.0
        } else {
            Scalar::INFINITY
        };
    }
    total_cardinality as Scalar / total_volume
}
