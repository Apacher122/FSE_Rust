//! Detailed validation diagnostics for constructed FSE indexes.

use std::collections::HashSet;

use crate::storage::FSEIndex;

/// Leaf node that violates the configured maximum leaf cardinality.
///
/// # Runtime Role
///
/// `LeafCardinalityViolation` identifies a terminal partition whose stored
/// record count exceeds the hard validation limit for the current build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafCardinalityViolation {
    /// Leaf node identifier.
    pub node_id: usize,

    /// Number of records represented by the leaf.
    pub cardinality: usize,

    /// Configured maximum leaf cardinality.
    pub max_leaf_size: usize,

    /// Number of records above the configured maximum.
    pub overflow_by: usize,
}

/// Invalid child reference found during hierarchy validation.
///
/// # Runtime Role
///
/// `InvalidChildReference` records a parent-child edge whose child identifier
/// does not point to an existing node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidChildReference {
    /// Parent node containing the invalid child reference.
    pub parent_id: usize,

    /// Child identifier that was outside the node list.
    pub child_id: usize,
}

/// Parent-child edge whose child bounds are not contained by the parent bounds.
///
/// # Runtime Role
///
/// `ParentChildBoundsViolation` identifies a hierarchy edge that violates the
/// bounding containment invariant required for safe pruning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParentChildBoundsViolation {
    /// Parent node identifier.
    pub parent_id: usize,

    /// Child node identifier.
    pub child_id: usize,
}

/// Detailed hierarchy topology diagnostics.
///
/// # Runtime Role
///
/// `HierarchyTopologyDiagnostics` expands the boolean hierarchy validation flag
/// into counts that explain which topology rule failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchyTopologyDiagnostics {
    /// Whether the root identifier points to an existing node.
    pub root_valid: bool,

    /// Invalid child references found in the hierarchy.
    pub invalid_child_references: Vec<InvalidChildReference>,

    /// Number of direct self-references found in child lists.
    pub self_reference_count: usize,

    /// Number of leaf nodes that incorrectly contain children.
    pub leaf_nodes_with_children_count: usize,

    /// Number of internal nodes that incorrectly contain no children.
    pub internal_nodes_without_children_count: usize,

    /// Number of nodes reachable from the root.
    pub reachable_node_count: usize,

    /// Number of nodes not reachable from the root.
    pub unreachable_node_count: usize,
}

/// Detailed validation diagnostics for an FSE index.
///
/// # Runtime Role
///
/// `IndexValidationDiagnostics` gives benchmark and test output enough detail
/// to explain why an index validation summary failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexValidationDiagnostics {
    /// Leaf cardinality violations.
    pub leaf_cardinality_violations: Vec<LeafCardinalityViolation>,

    /// Hierarchy topology diagnostics.
    pub hierarchy_topology: HierarchyTopologyDiagnostics,

    /// Parent-child bounds violations.
    pub parent_child_bounds_violations: Vec<ParentChildBoundsViolation>,
}

/// Builds detailed validation diagnostics for an FSE index.
///
/// # Runtime Role
///
/// This function complements the compact validation report. It is intended for
/// benchmark failure output and tests, not hot query execution.
pub fn index_validation_diagnostics(
    index: &FSEIndex,
    max_leaf_size: usize,
) -> IndexValidationDiagnostics {
    IndexValidationDiagnostics {
        leaf_cardinality_violations: leaf_cardinality_violations(index, max_leaf_size),
        hierarchy_topology: hierarchy_topology_diagnostics(index),
        parent_child_bounds_violations: parent_child_bounds_violations(index),
    }
}

fn leaf_cardinality_violations(
    index: &FSEIndex,
    max_leaf_size: usize,
) -> Vec<LeafCardinalityViolation> {
    index
        .nodes
        .iter()
        .filter(|node| node.is_leaf && node.cardinality > max_leaf_size)
        .map(|node| LeafCardinalityViolation {
            node_id: node.id,
            cardinality: node.cardinality,
            max_leaf_size,
            overflow_by: node.cardinality - max_leaf_size,
        })
        .collect()
}

fn hierarchy_topology_diagnostics(index: &FSEIndex) -> HierarchyTopologyDiagnostics {
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

fn parent_child_bounds_violations(index: &FSEIndex) -> Vec<ParentChildBoundsViolation> {
    let mut violations = Vec::new();

    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return violations;
    }

    for node in &index.nodes {
        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                continue;
            }

            let child = &index.nodes[*child_id];

            if !node.bounds.contains_bounds(&child.bounds) {
                violations.push(ParentChildBoundsViolation {
                    parent_id: node.id,
                    child_id: *child_id,
                });
            }
        }
    }

    violations
}
