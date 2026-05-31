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
    let available_capacity = matches.capacity().saturating_sub(matches.len());

    if shape.cardinality > available_capacity {
        matches.reserve_exact(shape.cardinality - available_capacity);
    }

    for_each_partial_leaf_reference_match(node, shape, query, &mut |reference| {
        matches.push(reference);
    });
}

/// Visits exact row references from a partially covered retained leaf.
///
/// # Runtime Role
///
/// Partial leaves still run exact predicate evaluation in coordinate space. The
/// visitor receives each matching row reference as soon as the exact predicate
/// accepts the reconstructed candidate.
pub(super) fn for_each_partial_leaf_reference_match<F>(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    visit: &mut F,
) where
    F: FnMut(QueryResultReference),
{
    #[cfg(any(test, debug_assertions))]
    {
        debug_assert_leaf_reconstruction_shape(node, shape);
        debug_assert_query_reconstruction_shape(query, shape);
    }

    match shape.dimensions {
        1 => for_each_partial_leaf_reference_match_1d(node, shape, query, visit),
        2 => for_each_partial_leaf_reference_match_2d(node, shape, query, visit),
        _ => for_each_partial_leaf_reference_match_generic(node, shape, query, visit),
    }
}

#[inline]
fn for_each_partial_leaf_reference_match_1d<F>(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    visit: &mut F,
) where
    F: FnMut(QueryResultReference),
{
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];

    for row_index in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row_index];

        if value_0 >= query_min_0 && value_0 <= query_max_0 {
            visit(QueryResultReference {
                node_id: shape.node_id,
                row_index,
            });
        }
    }
}

#[inline]
fn for_each_partial_leaf_reference_match_2d<F>(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    visit: &mut F,
) where
    F: FnMut(QueryResultReference),
{
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
            visit(QueryResultReference {
                node_id: shape.node_id,
                row_index,
            });
        }
    }
}

#[inline]
fn for_each_partial_leaf_reference_match_generic<F>(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    visit: &mut F,
) where
    F: FnMut(QueryResultReference),
{
    let mut values = vec![0.0; shape.dimensions];

    // still exact predicate evaluation just no owned row result
    for row_index in 0..shape.cardinality {
        for dimension in 0..shape.dimensions {
            values[dimension] =
                node.centroid[dimension] + node.residuals.dimensions[dimension][row_index];
        }

        if query.contains_values_prevalidated(&values, shape.dimensions) {
            visit(QueryResultReference {
                node_id: shape.node_id,
                row_index,
            });
        }
    }
}
