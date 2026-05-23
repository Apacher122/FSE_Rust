//! Count-only query execution.
//!
//! Count-only execution preserves the same geometric pruning and exact
//! predicate semantics as owned-result execution, but it does not allocate
//! returned `Vector` values for matching rows.

use crate::math::Scalar;
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::reports::{QueryCountReport, QueryExecutionStats};

/// Counts exact query matches without materializing owned result vectors.
///
/// # Runtime Role
///
/// This function is useful when a caller needs cardinality, existence checks,
/// or aggregate planning information without paying the owned-result
/// materialization cost of `execute_query`.
///
/// # Formal Reference
///
/// This preserves the staged execution model:
///
/// `Geometry -> Reconstruction -> Logic`
///
/// The difference from owned-result execution is that accepted rows increment a
/// counter instead of being converted into returned `Vector` values.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn count_query_matches(index: &FSEIndex, query: &QueryRegion) -> usize {
    count_query_matches_with_stats(index, query).matched_records
}

/// Counts exact query matches and returns execution statistics.
///
/// # Runtime Role
///
/// This function exposes the same structural work counters used by owned-result
/// query execution while avoiding final result allocation. `matched_records` is
/// duplicated at the top level for convenience and inside `stats` for
/// consistency with the existing execution report shape.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn count_query_matches_with_stats(index: &FSEIndex, query: &QueryRegion) -> QueryCountReport {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let root_classification = query.classify_bounds(&index.root_node().bounds);

    match root_classification {
        QueryBoundsClassification::Covered => {
            return count_fully_covered_index(index);
        }
        QueryBoundsClassification::Disjoint => {
            return count_root_disjoint_query(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path uses the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let total_records = index.root_node().cardinality;
    let reconstructed_records = traversal_report.stats.retained_candidate_records;
    let matched_records =
        count_retained_matches_without_results(index, query, &traversal_report.retained_leaves);

    QueryCountReport {
        matched_records,
        stats: QueryExecutionStats {
            visited_nodes: traversal_report.stats.visited_nodes,
            total_leaves: traversal_report.stats.total_leaves,
            retained_leaves: traversal_report.stats.retained_leaves,
            retained_leaf_ratio: traversal_report.stats.retained_leaf_ratio,
            total_records,
            reconstructed_records,
            matched_records,
            candidate_ratio: ratio_or_zero(reconstructed_records, total_records),
        },
    }
}

fn count_fully_covered_index(index: &FSEIndex) -> QueryCountReport {
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

fn count_root_disjoint_query(index: &FSEIndex) -> QueryCountReport {
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

fn count_retained_matches_without_results(
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

    // no owned result rows here, just the exact count
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

fn ratio_or_zero(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        0.0
    } else {
        numerator as Scalar / denominator as Scalar
    }
}
