//! Validation utilities for constructed FSE indexes.
//!
//! Compact validation is split by invariant so the high-level validation entry
//! point stays small and each construction rule has one focused module.

mod bounds;
mod leaf;
mod leaf_reconstruction;
mod leaf_records;
mod node_ids;
mod ownership;
mod report;
mod topology;

pub use bounds::validate_parent_child_bounds;
pub use leaf::validate_leaf_cardinality;
pub use leaf_reconstruction::validate_leaf_reconstruction_metadata;
pub use leaf_records::validate_leaf_record_bounds;
pub(crate) use leaf_records::value_is_inside_leaf_bounds;
pub use node_ids::validate_node_identifier_consistency;
pub use ownership::validate_leaf_ownership_cardinality;
pub use report::IndexValidationReport;
pub use topology::validate_hierarchy_topology;

use crate::storage::FSEIndex;

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
/// - node identifier consistency,
/// - leaf cardinality,
/// - leaf reconstruction metadata,
/// - leaf record bounded support,
/// - leaf ownership cardinality,
/// - hierarchy topology,
/// - parent-child bounding containment.
pub fn validate_index(index: &FSEIndex, max_leaf_size: usize) -> IndexValidationReport {
    IndexValidationReport {
        node_identifier_consistency_valid: validate_node_identifier_consistency(index),
        leaf_cardinality_valid: validate_leaf_cardinality(index, max_leaf_size),
        leaf_reconstruction_metadata_valid: validate_leaf_reconstruction_metadata(index),
        leaf_record_bounds_valid: validate_leaf_record_bounds(index),
        leaf_ownership_cardinality_valid: validate_leaf_ownership_cardinality(index),
        hierarchy_topology_valid: validate_hierarchy_topology(index),
        parent_child_bounds_valid: validate_parent_child_bounds(index),
    }
}
