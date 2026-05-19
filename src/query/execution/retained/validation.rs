//! Retained-leaf validation helpers.

use crate::query::RetainedLeaf;
use crate::storage::FSEIndex;

/// Validates that retained leaf records reference leaf partitions.
///
/// # Runtime Role
///
/// Classified traversal output should already be valid, but this keeps execution
/// helpers safe when tests or future callers construct retained leaves directly.
pub(crate) fn validate_retained_leaves(index: &FSEIndex, retained_leaves: &[RetainedLeaf]) {
    for retained_leaf in retained_leaves {
        let Some(node) = index.nodes.get(retained_leaf.node_id) else {
            panic!(
                "retained leaf id {} is outside index node range",
                retained_leaf.node_id
            );
        };

        assert!(
            node.is_leaf,
            "retained leaf id {} must reference a leaf partition",
            retained_leaf.node_id
        );
    }
}
