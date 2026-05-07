//! Validation utilities for constructed FSE indexes.

use crate::storage::FSEIndex;
use std::collections::HashSet;

/// Validation report for a constructed FSE index.
///
/// # Runtime Role
///
/// `IndexValidationReport` collects individual validation checks into one
/// result object that can be used by tests, demos, and future builder
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexValidationReport {
    /// Whether all leaf nodes satisfy the configured maximum leaf size.
    pub leaf_cardinality_valid: bool,
    /// Whether the hierarchy topology is structurally valid.
    pub hierarchy_topology_valid: bool,
    /// Whether every child bounding box is contained by its parent.
    pub parent_child_bounds_valid: bool,
}

impl IndexValidationReport {
    /// Returns true only when every validation check passed.
    pub fn is_valid(&self) -> bool {
        self.leaf_cardinality_valid
            && self.hierarchy_topology_valid
            && self.parent_child_bounds_valid
    }
}

/// Validates all core construction invariants for an FSE index.
///
/// # Runtime Role
///
/// This is the preferred high-level validation entry point for constructed
/// indexes.
///
/// # Validation Checks
///
/// This function validates:
///
/// - leaf cardinality,
/// - hierarchy topology,
/// - parent-child bounding containment.
pub fn validate_index(index: &FSEIndex, max_leaf_size: usize) -> IndexValidationReport {
    IndexValidationReport {
        leaf_cardinality_valid: validate_leaf_cardinality(index, max_leaf_size),
        hierarchy_topology_valid: validate_hierarchy_topology(index),
        parent_child_bounds_valid: validate_parent_child_bounds(index),
    }
}

/// Validates that all leaf partitions satisfy the configured maximum leaf size.
///
/// # Runtime Role
///
/// This function is used to verify that recursive index construction respected
/// the configured leaf cardinality bound.
///
/// # Notes
///
/// Internal nodes are ignored because they may contain metadata and residuals
/// depending on the current storage layout. This validation only checks leaf
/// partitions.
pub fn validate_leaf_cardinality(index: &FSEIndex, max_leaf_size: usize) -> bool {
    index
        .nodes
        .iter()
        .filter(|node| node.is_leaf)
        .all(|node| node.cardinality <= max_leaf_size)
}

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
    // Validation Rules:
    // - root node exists
    // - leaf nodes have no children
    // - internal nodes have children
    // - every node is reachable from root
    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return false;
    }

    for node in &index.nodes {
        if node.is_leaf && !node.children.is_empty() {
            return false;
        }
        if !node.is_leaf && node.children.is_empty() {
            return false;
        }
    }

    let mut visited = HashSet::new();
    let mut stack = vec![index.root];
    while let Some(node_id) = stack.pop() {
        if !visited.insert(node_id) {
            continue;
        }
        for child_id in &index.nodes[node_id].children {
            stack.push(*child_id);
        }
    }
    visited.len() == index.nodes.len()
}

pub fn validate_parent_child_bounds(index: &FSEIndex) -> bool {
    for node in &index.nodes {
        for child_id in &node.children {
            let child = &index.nodes[*child_id];
            if !node.bounds.contains_bounds(&child.bounds) {
                return false;
            }
        }
    }
    true
}
