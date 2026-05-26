//! Count-only retained-leaf execution.

use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::super::ratio::ratio_or_zero;
use super::super::reports::{QueryCountReport, QueryExecutionStats};

/// Counts every indexed row for a root-covered query.
///
/// # Runtime Role
///
/// If the root bounds are covered by the query, every indexed record is known to
/// match. Count-only execution can return the root cardinality without retained
/// leaf reconstruction or result materialization.
pub(super) fn count_fully_covered_index(index: &FSEIndex) -> QueryCountReport {
    let total_leaves = index.leaf_count();
    let total_records = index.root_node().cardinality;

    QueryCountReport {
        matched_records: total_records,
        stats: QueryExecutionStats {
            visited_nodes: 1,
            total_leaves,
            retained_leaves: total_leaves,
            retained_leaf_ratio: ratio_or_zero(total_leaves, total_leaves),
            total_records,
            reconstructed_records: total_records,
            matched_records: total_records,
            candidate_ratio: ratio_or_zero(total_records, total_records),
        },
    }
}

/// Returns an empty count report for a root-disjoint query.
///
/// # Runtime Role
///
/// Root-disjoint queries finish after the root metadata classification. They do
/// not retain leaves, reconstruct records, evaluate exact row predicates, or
/// materialize result rows.
pub(super) fn count_root_disjoint_query(index: &FSEIndex) -> QueryCountReport {
    QueryCountReport {
        matched_records: 0,
        stats: QueryExecutionStats {
            visited_nodes: 1,
            total_leaves: index.leaf_count(),
            retained_leaves: 0,
            retained_leaf_ratio: 0.0,
            total_records: index.root_node().cardinality,
            reconstructed_records: 0,
            matched_records: 0,
            candidate_ratio: 0.0,
        },
    }
}

/// Counts exact matches across traversal-retained leaves.
///
/// # Runtime Role
///
/// This is the count-only equivalent of retained-leaf result execution. Covered
/// leaves contribute their full cardinality. Partial leaves still run exact row
/// predicates, but matching rows only increment a counter.
pub(super) fn count_retained_matches_without_results(
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
