//! Leaf reconstruction metadata validation.

use crate::storage::{FSEIndex, LeafReconstructionShape};

/// Validates cached leaf reconstruction metadata.
///
/// # Validation Rule
///
/// Cached leaf counts, leaf identifiers, reconstruction shapes, and shape
/// lookup entries must match the leaf nodes stored in `FSEIndex::nodes`.
pub fn validate_leaf_reconstruction_metadata(index: &FSEIndex) -> bool {
    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return false;
    }

    let expected_leaf_node_ids = expected_leaf_node_ids(index);

    if index.leaf_count != expected_leaf_node_ids.len() {
        return false;
    }

    if index.leaf_node_ids.as_slice() != expected_leaf_node_ids.as_slice() {
        return false;
    }

    if index.leaf_reconstruction_shapes.len() != expected_leaf_node_ids.len() {
        return false;
    }

    if index.leaf_reconstruction_shapes_by_node.len() != index.nodes.len() {
        return false;
    }

    for (shape, node_id) in index
        .leaf_reconstruction_shapes
        .iter()
        .zip(&expected_leaf_node_ids)
    {
        if *shape != expected_shape(index, *node_id) {
            return false;
        }
    }

    index.nodes.iter().enumerate().all(|(node_id, node)| {
        let expected = node.is_leaf.then_some(expected_shape(index, node_id));

        index.leaf_reconstruction_shapes_by_node[node_id] == expected
    })
}

fn expected_leaf_node_ids(index: &FSEIndex) -> Vec<usize> {
    index
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(node_id, node)| node.is_leaf.then_some(node_id))
        .collect()
}

fn expected_shape(index: &FSEIndex, node_id: usize) -> LeafReconstructionShape {
    let node = &index.nodes[node_id];

    LeafReconstructionShape::new(node_id, node.dimensions(), node.residuals.cardinality())
}
