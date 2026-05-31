//! Covered retained-leaf reference materialization.

use crate::storage::{FSEIndex, LeafReconstructionShape};

use super::super::super::reports::QueryResultReference;

/// Visits every row reference from every leaf in a root-covered query.
///
/// # Runtime Role
///
/// Root-covered reference execution does not need traversal-retained leaf
/// records. The root bounds already prove every indexed row is a match, so this
/// function visits references from the cached leaf reconstruction shapes.
pub(in crate::query::execution) fn for_each_fully_covered_index_reference<F>(
    index: &FSEIndex,
    mut visit: F,
) where
    F: FnMut(QueryResultReference),
{
    for shape in index.leaf_reconstruction_shapes() {
        for_each_covered_leaf_reference(*shape, &mut visit);
    }
}

/// Appends every row reference from every leaf in a root-covered query.
///
/// # Runtime Role
///
/// This helper preserves the vector-backed reference output contract while
/// sharing reference traversal with the visitor output contract.
pub(in crate::query::execution::references) fn append_fully_covered_index_references(
    index: &FSEIndex,
    matches: &mut Vec<QueryResultReference>,
) {
    for_each_fully_covered_index_reference(index, |reference| {
        matches.push(reference);
    });
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

    for_each_covered_leaf_reference(shape, &mut |reference| {
        matches.push(reference);
    });
}

/// Visits every row reference from a covered retained leaf.
///
/// # Runtime Role
///
/// Covered leaves can stream exact row references directly because geometric
/// containment has already established that every row in the leaf is accepted.
pub(super) fn for_each_covered_leaf_reference<F>(shape: LeafReconstructionShape, visit: &mut F)
where
    F: FnMut(QueryResultReference),
{
    // covered geometry means the row references are already exact matches
    for row_index in 0..shape.cardinality {
        visit(QueryResultReference {
            node_id: shape.node_id,
            row_index,
        });
    }
}
