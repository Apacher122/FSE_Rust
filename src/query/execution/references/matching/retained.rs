//! Retained-leaf reference matching orchestration.

use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::super::super::reports::QueryResultReference;
use super::{
    covered::append_covered_leaf_references, covered::for_each_covered_leaf_reference,
    partial::append_partial_leaf_reference_matches, partial::for_each_partial_leaf_reference_match,
};

/// Visits exact row references for traversal-retained leaves.
///
/// # Runtime Role
///
/// This function performs the streaming reference equivalent of retained-leaf
/// execution. Covered leaves visit every row reference directly. Partial leaves
/// reconstruct only enough coordinate data to run exact predicate checks before
/// visiting a row reference.
pub(in crate::query::execution) fn for_each_retained_reference_match<F>(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    mut visit: F,
) where
    F: FnMut(QueryResultReference),
{
    for retained_leaf in retained_leaves {
        let node = &index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(index);

        for_each_retained_leaf_reference_match(
            node,
            shape,
            retained_leaf.coverage,
            query,
            &mut visit,
        );
    }
}

/// Appends exact row references for traversal-retained leaves.
///
/// # Runtime Role
///
/// This function preserves the vector-backed reference output contract while
/// sharing retained-leaf matching with the visitor output contract.
pub(in crate::query::execution::references) fn append_retained_reference_matches(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    matches: &mut Vec<QueryResultReference>,
) {
    for retained_leaf in retained_leaves {
        let node = &index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(index);

        append_retained_leaf_reference_matches(node, shape, retained_leaf.coverage, query, matches);
    }
}

fn append_retained_leaf_reference_matches(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    query: &QueryRegion,
    matches: &mut Vec<QueryResultReference>,
) {
    match coverage {
        RetainedLeafCoverage::Covered => append_covered_leaf_references(shape, matches),
        RetainedLeafCoverage::Partial => {
            append_partial_leaf_reference_matches(node, shape, query, matches)
        }
    }
}

fn for_each_retained_leaf_reference_match<F>(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    query: &QueryRegion,
    visit: &mut F,
) where
    F: FnMut(QueryResultReference),
{
    match coverage {
        RetainedLeafCoverage::Covered => for_each_covered_leaf_reference(shape, visit),
        RetainedLeafCoverage::Partial => {
            for_each_partial_leaf_reference_match(node, shape, query, visit)
        }
    }
}
