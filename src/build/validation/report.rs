//! Compact index validation report type.

/// Validation report for a constructed FSE index.
///
/// # Runtime Role
///
/// `IndexValidationReport` collects individual validation checks into one
/// result object that can be used by tests, demos, and future builder diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexValidationReport {
    /// Whether node identifiers match their index positions.
    pub node_identifier_consistency_valid: bool,
    /// Whether all leaf nodes satisfy the configured maximum leaf size.
    pub leaf_cardinality_valid: bool,
    /// Whether cached leaf reconstruction metadata matches the leaf nodes.
    pub leaf_reconstruction_metadata_valid: bool,
    /// Whether reconstructed leaf rows are contained by their leaf bounds.
    pub leaf_record_bounds_valid: bool,
    /// Whether leaf ownership cardinalities match internal node cardinalities.
    pub leaf_ownership_cardinality_valid: bool,
    /// Whether the hierarchy topology is structurally valid.
    pub hierarchy_topology_valid: bool,
    /// Whether every child bounding box is contained by its parent.
    pub parent_child_bounds_valid: bool,
}

impl IndexValidationReport {
    /// Returns true only when every validation check passed.
    pub fn is_valid(&self) -> bool {
        self.node_identifier_consistency_valid
            && self.leaf_cardinality_valid
            && self.leaf_reconstruction_metadata_valid
            && self.leaf_record_bounds_valid
            && self.leaf_ownership_cardinality_valid
            && self.hierarchy_topology_valid
            && self.parent_child_bounds_valid
    }
}
