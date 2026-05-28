//! Detailed validation diagnostics for constructed FSE indexes.
//!
//! This module expands compact index validation booleans into concrete
//! diagnostic records that can explain build-validation failures.

mod bounds;
mod leaf;
mod topology;
mod types;

pub use types::{
    HierarchyTopologyDiagnostics, IndexValidationDiagnostics, InvalidChildReference,
    LeafCardinalityViolation, ParentChildBoundsViolation,
};

use crate::storage::FSEIndex;

use self::bounds::parent_child_bounds_violations;
use self::leaf::leaf_cardinality_violations;
use self::topology::hierarchy_topology_diagnostics;

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
