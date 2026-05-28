//! Hierarchy topology validation diagnostics.

use std::collections::HashSet;

use crate::storage::FSEIndex;

use super::types::{HierarchyTopologyDiagnostics, InvalidChildReference};

pub(super) fn hierarchy_topology_diagnostics(index: &FSEIndex) -> HierarchyTopologyDiagnostics {
    let root_valid = !index.nodes.is_empty() && index.root < index.nodes.len();

    let mut invalid_child_references = Vec::new();
    let mut self_reference_count = 0;
    let mut leaf_nodes_with_children_count = 0;
    let mut internal_nodes_without_children_count = 0;

    for node in &index.nodes {
        if node.is_leaf && !node.children.is_empty() {
            leaf_nodes_with_children_count += 1;
        }

        if !node.is_leaf && node.children.is_empty() {
            internal_nodes_without_children_count += 1;
        }

        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                invalid_child_references.push(InvalidChildReference {
                    parent_id: node.id,
                    child_id: *child_id,
                });
                continue;
            }

            if *child_id == node.id {
                self_reference_count += 1;
            }
        }
    }

    let mut visited = HashSet::new();

    if root_valid {
        let mut stack = vec![index.root];

        while let Some(node_id) = stack.pop() {
            if !visited.insert(node_id) {
                continue;
            }

            let node = &index.nodes[node_id];

            for child_id in &node.children {
                if *child_id < index.nodes.len() {
                    stack.push(*child_id);
                }
            }
        }
    }

    HierarchyTopologyDiagnostics {
        root_valid,
        invalid_child_references,
        self_reference_count,
        leaf_nodes_with_children_count,
        internal_nodes_without_children_count,
        reachable_node_count: visited.len(),
        unreachable_node_count: index.nodes.len().saturating_sub(visited.len()),
    }
}
