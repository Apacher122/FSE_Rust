//! Reference-result validation helpers.

use crate::storage::{FSEIndex, PartitionNode};

use super::super::super::reports::QueryResultReference;

/// Returns the leaf node targeted by a query result reference.
///
/// # Runtime Role
///
/// Reference-result reconstruction is intentionally deferred, so every supplied
/// reference must be validated before residual storage is read.
///
/// # Panics
///
/// Panics when the referenced node does not exist or does not identify a leaf
/// partition.
pub(super) fn reference_leaf_node(
    index: &FSEIndex,
    reference: QueryResultReference,
) -> &PartitionNode {
    let node = index.nodes.get(reference.node_id).unwrap_or_else(|| {
        panic!(
            "query result reference node id {} is outside the index",
            reference.node_id
        )
    });

    // stale references should fail before row reconstruction touches storage
    assert!(
        node.is_leaf,
        "query result reference node id {} must reference a leaf partition",
        reference.node_id
    );

    node
}
