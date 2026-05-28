//! Parent-child bounds validation.

use crate::storage::FSEIndex;

/// Validates that every child bounding box is contained by its parent.
///
/// # Runtime Role
///
/// This function verifies the hierarchy containment invariant required for
/// recursive pruning correctness.
///
/// # Validation Rule
///
/// For every parent-child edge in the hierarchy:
///
/// `B_child` must be contained within `B_parent`.
pub fn validate_parent_child_bounds(index: &FSEIndex) -> bool {
    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return false;
    }

    for node in &index.nodes {
        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                return false;
            }

            let child = &index.nodes[*child_id];

            if !node.bounds.contains_bounds(&child.bounds) {
                return false;
            }
        }
    }

    true
}
