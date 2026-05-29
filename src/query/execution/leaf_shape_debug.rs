//! Debug-only retained-leaf reconstruction shape assertions.
//!
//! These helpers centralize shape consistency checks shared by owned-result and
//! reference-result retained-leaf execution paths.

use crate::query::QueryRegion;
use crate::storage::{LeafReconstructionShape, PartitionNode};

/// Verifies that cached leaf reconstruction metadata matches the leaf node.
///
/// # Runtime Role
///
/// Cached leaf shape metadata is created during index construction and reused
/// during query execution. These assertions catch shape drift in debug and test
/// builds without adding release-path validation overhead.
pub(crate) fn debug_assert_leaf_reconstruction_shape(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
) {
    debug_assert_eq!(
        node.dimensions(),
        shape.dimensions,
        "cached leaf dimensionality should match node dimensionality"
    );
    debug_assert_eq!(
        node.residuals.cardinality(),
        shape.cardinality,
        "cached leaf cardinality should match residual cardinality"
    );
}

/// Verifies that a query and cached leaf shape use the same dimensionality.
///
/// # Runtime Role
///
/// Partial retained-leaf execution reconstructs rows before exact predicate
/// evaluation. The query and reconstructed rows must share dimensionality for
/// prevalidated predicate checks to remain sound.
pub(crate) fn debug_assert_query_reconstruction_shape(
    query: &QueryRegion,
    shape: LeafReconstructionShape,
) {
    // debug only because the build path owns the real invariant
    debug_assert_eq!(
        query.dimensions(),
        shape.dimensions,
        "query dimensionality should match retained leaf dimensionality"
    );
}
