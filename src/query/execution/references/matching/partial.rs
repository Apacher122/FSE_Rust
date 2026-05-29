//! Partially covered retained-leaf reference matching.

use crate::query::QueryRegion;
use crate::storage::{LeafReconstructionShape, PartitionNode};

use super::super::super::reports::QueryResultReference;

#[cfg(any(test, debug_assertions))]
use super::super::super::leaf_shape_debug::{
    debug_assert_leaf_reconstruction_shape, debug_assert_query_reconstruction_shape,
};

/// Appends exact row references from a partially covered retained leaf.
///
/// # Runtime Role
///
/// Partial leaves still require exact predicate evaluation. Matching rows are
/// returned as leaf/row references instead of owned vectors.
pub(super) fn append_partial_leaf_reference_matches(
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
