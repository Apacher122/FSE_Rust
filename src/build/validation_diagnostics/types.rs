//! Validation diagnostic record types.

use crate::math::Scalar;

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

/// Reconstructed leaf row value outside its leaf bounds.
///
/// # Runtime Role
///
/// `LeafRecordBoundsViolation` identifies the row and dimension that failed
/// the leaf bounded-support check.
#[derive(Clone, Debug, PartialEq)]
pub struct LeafRecordBoundsViolation {
    /// Leaf node identifier.
    pub node_id: usize,

    /// Residual row index inside the leaf.
    pub row: usize,

    /// Coordinate dimension that failed the bounds check.
    pub dimension: usize,

    /// Reconstructed coordinate value.
    pub value: Scalar,

    /// Minimum allowed coordinate value for the dimension.
    pub minimum: Scalar,

    /// Maximum allowed coordinate value for the dimension.
    pub maximum: Scalar,
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

/// Node whose stored identifier does not match its index position.
///
/// # Runtime Role
///
/// `NodeIdentifierMismatch` records a node whose `PartitionNode::id` differs
/// from its position in `FSEIndex::nodes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeIdentifierMismatch {
    /// Expected node identifier from the node-list position.
    pub expected_id: usize,

    /// Stored node identifier.
    pub stored_id: usize,
}

/// Node whose observed parent count does not match the ownership rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafOwnershipParentCountViolation {
    /// Node identifier.
    pub node_id: usize,

    /// Number of parent references found for the node.
    pub parent_count: usize,

    /// Required parent count for the node.
    pub expected_parent_count: usize,
}

/// Node whose declared cardinality does not match its owned leaf cardinality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafOwnershipCardinalityViolation {
    /// Node identifier.
    pub node_id: usize,

    /// Declared node cardinality.
    pub cardinality: usize,

    /// Cardinality obtained by summing the node's owned leaf subtrees.
    pub owned_leaf_cardinality: usize,
}

/// Detailed leaf ownership cardinality diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafOwnershipCardinalityDiagnostics {
    /// Nodes with an invalid number of parents.
    pub parent_count_violations: Vec<LeafOwnershipParentCountViolation>,

    /// Nodes whose declared cardinality differs from owned leaf cardinality.
    pub cardinality_violations: Vec<LeafOwnershipCardinalityViolation>,

    /// Nodes that are not reachable from the root.
    pub unowned_node_ids: Vec<usize>,
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
#[derive(Clone, Debug, PartialEq)]
pub struct IndexValidationDiagnostics {
    /// Node identifier mismatches.
    pub node_identifier_mismatches: Vec<NodeIdentifierMismatch>,

    /// Leaf cardinality violations.
    pub leaf_cardinality_violations: Vec<LeafCardinalityViolation>,

    /// Reconstructed leaf rows outside their leaf bounds.
    pub leaf_record_bounds_violations: Vec<LeafRecordBoundsViolation>,

    /// Leaf ownership cardinality diagnostics.
    pub leaf_ownership_cardinality: LeafOwnershipCardinalityDiagnostics,

    /// Hierarchy topology diagnostics.
    pub hierarchy_topology: HierarchyTopologyDiagnostics,

    /// Parent-child bounds violations.
    pub parent_child_bounds_violations: Vec<ParentChildBoundsViolation>,
}
