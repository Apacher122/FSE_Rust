//! Node identifier consistency validation diagnostics.

use crate::storage::FSEIndex;

use super::types::NodeIdentifierMismatch;

pub(super) fn node_identifier_mismatches(index: &FSEIndex) -> Vec<NodeIdentifierMismatch> {
    index
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(expected_id, node)| {
            (node.id != expected_id).then_some(NodeIdentifierMismatch {
                expected_id,
                stored_id: node.id,
            })
        })
        .collect()
}
