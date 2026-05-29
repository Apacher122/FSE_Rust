//! Covered retained-leaf reference materialization.

use crate::storage::{FSEIndex, LeafReconstructionShape};

use super::super::super::reports::QueryResultReference;

/// Appends every row reference from every leaf in a root-covered query.
///
/// # Runtime Role
///
/// Root-covered reference execution does not need traversal-retained leaf
/// records. The root bounds already prove every indexed row is a match, so this
/// function appends references from the cached leaf reconstruction shapes.
pub(in crate::query::execution::references) fn append_fully_covered_index_references(
    index: &FSEIndex,
    matches: &mut Vec<QueryResultReference>,
) {
    for shape in index.leaf_reconstruction_shapes() {
        append_covered_leaf_references(*shape, matches);
    }
}

/// Appends every row reference from a covered retained leaf.
///
/// # Runtime Role
///
/// A covered retained leaf has already passed geometric containment, so exact
/// predicate checks are unnecessary for every row in the leaf.
pub(super) fn append_covered_leaf_references(
    shape: LeafReconstructionShape,
    matches: &mut Vec<QueryResultReference>,
) {
    let available_capacity = matches.capacity().saturating_sub(matches.len());

    if shape.cardinality > available_capacity {
        matches.reserve_exact(shape.cardinality - available_capacity);
    }

    // covered geometry means the row references are already exact matches
    for row_index in 0..shape.cardinality {
        matches.push(QueryResultReference {
            node_id: shape.node_id,
            row_index,
        });
    }
}
