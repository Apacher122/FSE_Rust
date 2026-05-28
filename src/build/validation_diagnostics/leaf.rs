//! Leaf-cardinality validation diagnostics.

use crate::storage::FSEIndex;

use super::types::LeafCardinalityViolation;

pub(super) fn leaf_cardinality_violations(
    index: &FSEIndex,
    max_leaf_size: usize,
) -> Vec<LeafCardinalityViolation> {
    index
        .nodes
        .iter()
        .filter(|node| node.is_leaf && node.cardinality > max_leaf_size)
        .map(|node| LeafCardinalityViolation {
            node_id: node.id,
            cardinality: node.cardinality,
            max_leaf_size,
            overflow_by: node.cardinality - max_leaf_size,
        })
        .collect()
}
