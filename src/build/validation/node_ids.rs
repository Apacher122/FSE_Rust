//! Node identifier consistency validation.

use crate::storage::FSEIndex;

/// Validates that node identifiers match their position in the index.
///
/// # Validation Rule
///
/// Every `PartitionNode::id` must equal its position in `FSEIndex::nodes`.
/// Child references, retained-leaf references, and reconstruction shape caches
/// use node-list positions as stable runtime identifiers.
pub fn validate_node_identifier_consistency(index: &FSEIndex) -> bool {
    !index.nodes.is_empty()
        && index
            .nodes
            .iter()
            .enumerate()
            .all(|(node_id, node)| node.id == node_id)
}
