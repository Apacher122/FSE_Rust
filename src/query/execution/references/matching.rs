//! Reference-result retained-leaf matching.

use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::super::reports::QueryResultReference;

#[cfg(any(test, debug_assertions))]
use super::super::leaf_shape_debug::{
    debug_assert_leaf_reconstruction_shape, debug_assert_query_reconstruction_shape,
};

/// Appends exact row references for traversal-retained leaves.
///
/// # Runtime Role
///
/// This function performs the reference-result equivalent of retained-leaf
/// execution. Covered leaves append every row reference directly. Partial leaves
/// reconstruct only enough coordinate data to run exact predicate checks before
/// recording a row reference.
pub(super) fn append_retained_reference_matches(
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

    for row_index in 0..shape.cardinality {
        matches.push(QueryResultReference {
            node_id: shape.node_id,
            row_index,
        });
    }
}

fn append_partial_leaf_reference_matches(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    matches: &mut Vec<QueryResultReference>,
) {
    #[cfg(any(test, debug_assertions))]
    {
        debug_assert_leaf_reconstruction_shape(node, shape);
        debug_assert_query_reconstruction_shape(query, shape);
    }

    match shape.dimensions {
        1 => append_partial_leaf_reference_matches_1d(node, shape, query, matches),
        2 => append_partial_leaf_reference_matches_2d(node, shape, query, matches),
        _ => append_partial_leaf_reference_matches_generic(node, shape, query, matches),
    }
}

#[inline]
fn append_partial_leaf_reference_matches_1d(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    matches: &mut Vec<QueryResultReference>,
) {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];

    for row_index in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row_index];

        if value_0 >= query_min_0 && value_0 <= query_max_0 {
            matches.push(QueryResultReference {
                node_id: shape.node_id,
                row_index,
            });
        }
    }
}

#[inline]
fn append_partial_leaf_reference_matches_2d(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    matches: &mut Vec<QueryResultReference>,
) {
    let centroid_0 = node.centroid[0];
    let centroid_1 = node.centroid[1];

    let residual_0 = &node.residuals.dimensions[0];
    let residual_1 = &node.residuals.dimensions[1];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];
    let query_min_1 = query.min[1];
    let query_max_1 = query.max[1];

    for row_index in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row_index];

        if value_0 < query_min_0 || value_0 > query_max_0 {
            continue;
        }

        let value_1 = centroid_1 + residual_1[row_index];

        if value_1 >= query_min_1 && value_1 <= query_max_1 {
            matches.push(QueryResultReference {
                node_id: shape.node_id,
                row_index,
            });
        }
    }
}

#[inline]
fn append_partial_leaf_reference_matches_generic(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    matches: &mut Vec<QueryResultReference>,
) {
    let mut values = vec![0.0; shape.dimensions];

    // still exact predicate evaluation just no owned row result
    for row_index in 0..shape.cardinality {
        for dimension in 0..shape.dimensions {
            values[dimension] =
                node.centroid[dimension] + node.residuals.dimensions[dimension][row_index];
        }

        if query.contains_values_prevalidated(&values, shape.dimensions) {
            matches.push(QueryResultReference {
                node_id: shape.node_id,
                row_index,
            });
        }
    }
}
