//! Hierarchy topology validation.

use std::collections::HashSet;

use crate::storage::FSEIndex;

/// Validates the structural topology of an FSE index.
///
/// # Runtime Role
///
/// This function checks whether the hierarchy can be safely traversed from the
/// configured root without invalid child references or inconsistent node flags.
///
/// # Validation Rules
///
/// The topology is valid when:
///
/// - the root node exists,
/// - every child ID refers to an existing node,
/// - leaf nodes have no children,
/// - internal nodes have at least one child,
/// - a node does not directly reference itself,
/// - every node is reachable from the root.
pub fn validate_hierarchy_topology(index: &FSEIndex) -> bool {
    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return false;
    }

    for (node_id, node) in index.nodes.iter().enumerate() {
        if node.is_leaf && !node.children.is_empty() {
            return false;
        }
        if !node.is_leaf && node.children.is_empty() {
            return false;
        }
        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                return false;
            }

            if *child_id == node_id {
                return false;
            }
        }
    }

    let mut visited = HashSet::new();
    let mut stack = vec![index.root];

    while let Some(node_id) = stack.pop() {
        if !visited.insert(node_id) {
            continue;
        }

        let node = &index.nodes[node_id];

        for child_id in &node.children {
            stack.push(*child_id);
        }
    }

    visited.len() == index.nodes.len()
}
