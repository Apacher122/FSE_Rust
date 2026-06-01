//! Leaf reconstruction metadata validation diagnostics.

use crate::storage::{FSEIndex, LeafReconstructionShape};

use super::types::{
    LeafReconstructionLeafCountMismatch, LeafReconstructionMetadataDiagnostics,
    LeafReconstructionShapeListLengthMismatch, LeafReconstructionShapeListMismatch,
    LeafReconstructionShapeLookupLengthMismatch, LeafReconstructionShapeLookupMismatch,
};

pub(super) fn leaf_reconstruction_metadata_diagnostics(
    index: &FSEIndex,
) -> LeafReconstructionMetadataDiagnostics {
    let expected_leaf_node_ids = expected_leaf_node_ids(index);

    LeafReconstructionMetadataDiagnostics {
        leaf_count_mismatch: leaf_count_mismatch(index, expected_leaf_node_ids.len()),
        shape_list_mismatch: shape_list_mismatch(index, &expected_leaf_node_ids),
        shape_list_length_mismatch: shape_list_length_mismatch(index, expected_leaf_node_ids.len()),
        shape_lookup_length_mismatch: shape_lookup_length_mismatch(index),
        shape_lookup_mismatches: shape_lookup_mismatches(index),
    }
}

fn leaf_count_mismatch(
    index: &FSEIndex,
    expected_leaf_count: usize,
) -> Option<LeafReconstructionLeafCountMismatch> {
    (index.leaf_count != expected_leaf_count).then_some(LeafReconstructionLeafCountMismatch {
        expected_leaf_count,
        cached_leaf_count: index.leaf_count,
    })
}

fn shape_list_mismatch(
    index: &FSEIndex,
    expected_leaf_node_ids: &[usize],
) -> Option<LeafReconstructionShapeListMismatch> {
    let expected_shapes: Vec<LeafReconstructionShape> = expected_leaf_node_ids
        .iter()
        .map(|node_id| expected_shape(index, *node_id))
        .collect();

    (index.leaf_node_ids.as_slice() != expected_leaf_node_ids
        || index.leaf_reconstruction_shapes.as_slice() != expected_shapes.as_slice())
    .then_some(LeafReconstructionShapeListMismatch {
        expected_leaf_node_ids: expected_leaf_node_ids.to_vec(),
        cached_leaf_node_ids: index.leaf_node_ids.clone(),
        expected_shapes,
        cached_shapes: index.leaf_reconstruction_shapes.clone(),
    })
}

fn shape_list_length_mismatch(
    index: &FSEIndex,
    expected_shape_count: usize,
) -> Option<LeafReconstructionShapeListLengthMismatch> {
    (index.leaf_reconstruction_shapes.len() != expected_shape_count).then_some(
        LeafReconstructionShapeListLengthMismatch {
            expected_shape_count,
            cached_shape_count: index.leaf_reconstruction_shapes.len(),
        },
    )
}

fn shape_lookup_length_mismatch(
    index: &FSEIndex,
) -> Option<LeafReconstructionShapeLookupLengthMismatch> {
    let expected_lookup_len = index.nodes.len();
    let cached_lookup_len = index.leaf_reconstruction_shapes_by_node.len();

    (cached_lookup_len != expected_lookup_len).then_some(
        LeafReconstructionShapeLookupLengthMismatch {
            expected_lookup_len,
            cached_lookup_len,
        },
    )
}

fn shape_lookup_mismatches(index: &FSEIndex) -> Vec<LeafReconstructionShapeLookupMismatch> {
    let mut mismatches = Vec::new();
    let lookup_len = index.leaf_reconstruction_shapes_by_node.len();

    for (node_id, node) in index.nodes.iter().enumerate() {
        let expected_shape = node.is_leaf.then_some(expected_shape(index, node_id));
        let cached_shape = if node_id < lookup_len {
            index.leaf_reconstruction_shapes_by_node[node_id]
        } else {
            None
        };

        if cached_shape != expected_shape {
            mismatches.push(LeafReconstructionShapeLookupMismatch {
                node_id,
                expected_shape,
                cached_shape,
            });
        }
    }

    mismatches
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
