//! Validation utilities for constructed FSE indexes.
//!
//! Compact validation is split by invariant so the high-level validation entry
//! point stays small and each construction rule has one focused module.

mod bounds;
mod leaf;
mod report;
mod topology;

pub use bounds::validate_parent_child_bounds;
pub use leaf::validate_leaf_cardinality;
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
