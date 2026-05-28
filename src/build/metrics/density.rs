//! Index and partition density metrics.

use crate::math::Scalar;
use crate::storage::{FSEIndex, PartitionNode};

use super::types::IndexStructureMetrics;

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
    index_structure_metrics(index).index_density
}

/// Computes aggregate structural metrics for an FSE index.
///
/// # Runtime Role
///
/// This function summarizes the hierarchy shape produced by the builder. The
/// benchmark layer uses it to explain whether build-policy changes are creating
/// tighter leaves or simply adding traversal nodes.
///
/// # Formal Reference
///
/// Since query cost depends on traversal work and reconstructed records, these
/// structural metrics provide the bridge between construction policy and query
/// execution behavior.
pub fn index_structure_metrics(index: &FSEIndex) -> IndexStructureMetrics {
    let leaves: Vec<&PartitionNode> = index.nodes.iter().filter(|node| node.is_leaf).collect();

    let leaf_count = leaves.len();
    let internal_node_count = index.nodes.len().saturating_sub(leaf_count);
    let total_leaf_cardinality: usize = leaves.iter().map(|node| node.cardinality).sum();
    let min_leaf_cardinality = leaves
        .iter()
        .map(|node| node.cardinality)
        .min()
        .unwrap_or(0);
    let max_leaf_cardinality = leaves
        .iter()
        .map(|node| node.cardinality)
        .max()
        .unwrap_or(0);

    let total_leaf_volume: Scalar = leaves.iter().map(|node| node.bounds.volume()).sum();
    let zero_volume_leaf_count = leaves
        .iter()
        .filter(|node| node.bounds.volume() == 0.0)
        .count();

    let average_leaf_cardinality = if leaf_count == 0 {
        0.0
    } else {
        total_leaf_cardinality as Scalar / leaf_count as Scalar
    };

    let average_leaf_volume = if leaf_count == 0 {
        0.0
    } else {
        total_leaf_volume / leaf_count as Scalar
    };

    let index_density = if total_leaf_volume == 0.0 {
        if total_leaf_cardinality == 0 {
            0.0
        } else {
            Scalar::INFINITY
        }
    } else {
        total_leaf_cardinality as Scalar / total_leaf_volume
    };

    // this is the build shape signal no more guessing
    IndexStructureMetrics {
        node_count: index.nodes.len(),
        leaf_count,
        internal_node_count,
        total_leaf_cardinality,
        min_leaf_cardinality,
        max_leaf_cardinality,
        average_leaf_cardinality,
        total_leaf_volume,
        average_leaf_volume,
        index_density,
        zero_volume_leaf_count,
    }
}
