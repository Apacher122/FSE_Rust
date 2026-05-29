//! Retained-leaf reference matching orchestration.

use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::super::super::reports::QueryResultReference;
use super::{
    covered::append_covered_leaf_references, partial::append_partial_leaf_reference_matches,
};

/// Appends exact row references for traversal-retained leaves.
///
/// # Runtime Role
///
/// This function performs the reference-result equivalent of retained-leaf
/// execution. Covered leaves append every row reference directly. Partial leaves
/// reconstruct only enough coordinate data to run exact predicate checks before
/// recording a row reference.
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
