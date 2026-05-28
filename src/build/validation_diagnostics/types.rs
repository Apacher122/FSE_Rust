//! Validation diagnostic record types.

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
