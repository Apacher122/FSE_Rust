//! Compact index validation report type.

/// Validation report for a constructed FSE index.
///
/// # Runtime Role
///
/// `IndexValidationReport` collects individual validation checks into one
/// result object that can be used by tests, demos, and future builder diagnostics.
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
