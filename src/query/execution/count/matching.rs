//! Count-only retained-leaf matching.

use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

/// Counts exact matches across traversal-retained leaves.
///
/// # Runtime Role
///
/// This is the count-only equivalent of retained-leaf result execution. Covered
/// leaves contribute their full cardinality. Partial leaves still run exact row
/// predicates, but matching rows only increment a counter.
pub(crate) fn count_retained_matches_without_results(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
) -> usize {
    let mut matched_records = 0;

    for retained_leaf in retained_leaves {
        let node = &index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(index);

        matched_records +=
            count_retained_leaf_matches_without_results(node, shape, retained_leaf.coverage, query);
    }

    matched_records
}

fn count_retained_leaf_matches_without_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    query: &QueryRegion,
) -> usize {
    match coverage {
        RetainedLeafCoverage::Covered => shape.cardinality,
        RetainedLeafCoverage::Partial => {
            count_partial_retained_leaf_matches_without_results(node, shape, query)
        }
    }
}

fn count_partial_retained_leaf_matches_without_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> usize {
    match shape.dimensions {
        1 => count_partial_retained_leaf_matches_1d_without_results(node, shape, query),
        2 => count_partial_retained_leaf_matches_2d_without_results(node, shape, query),
        _ => count_partial_retained_leaf_matches_generic_without_results(node, shape, query),
    }
}

fn count_partial_retained_leaf_matches_1d_without_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> usize {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];

    let mut matched_records = 0;

    for row in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row];

        if value_0 >= query_min_0 && value_0 <= query_max_0 {
            matched_records += 1;
        }
    }

    matched_records
}

fn count_partial_retained_leaf_matches_2d_without_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> usize {
    let centroid_0 = node.centroid[0];
    let centroid_1 = node.centroid[1];

    let residual_0 = &node.residuals.dimensions[0];
    let residual_1 = &node.residuals.dimensions[1];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];
    let query_min_1 = query.min[1];
    let query_max_1 = query.max[1];

    let mut matched_records = 0;

    // no owned result rows here just the exact count
    for row in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row];

        if value_0 < query_min_0 || value_0 > query_max_0 {
            continue;
        }

        let value_1 = centroid_1 + residual_1[row];

        if value_1 >= query_min_1 && value_1 <= query_max_1 {
            matched_records += 1;
        }
    }

    matched_records
}

fn count_partial_retained_leaf_matches_generic_without_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> usize {
    let mut matched_records = 0;
    let mut values = vec![0.0; shape.dimensions];

    for row in 0..shape.cardinality {
        for dimension in 0..shape.dimensions {
            values[dimension] =
                node.centroid[dimension] + node.residuals.dimensions[dimension][row];
        }

        if query.contains_values_prevalidated(&values, shape.dimensions) {
            matched_records += 1;
        }
    }

    matched_records
}
