//! Reference-result query execution.
//!
//! This module provides an exact query output contract that returns references
//! to matching residual rows instead of materializing owned `Vector` values.

use crate::math::Scalar;
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::reports::{
    QueryExecutionStats, QueryReferenceReport, QueryResultReference, result_capacity_hint,
};

/// Executes a query and returns exact matching row references.
///
/// # Runtime Role
///
/// This function preserves the same pruning and exact predicate semantics as
/// owned-result query execution, but returns leaf/row references instead of
/// allocating one owned `Vector` per match.
///
/// # Formal Reference
///
/// This preserves the required execution order:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// The reconstruction step is still used for exact predicate evaluation on
/// partially covered leaves. The difference is that accepted rows are returned
/// as references to indexed residual storage.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_references(
    index: &FSEIndex,
    query: &QueryRegion,
) -> Vec<QueryResultReference> {
    execute_query_references_with_stats(index, query).matches
}

/// Executes a query and returns exact matching row references with stats.
///
/// # Runtime Role
///
/// `QueryReferenceReport` gives the benchmark layer a third output contract
/// between owned-result execution and count-only execution:
///
/// - owned-result execution materializes `Vec<Vector>`,
/// - reference-result execution materializes row references,
/// - count-only execution materializes only exact cardinality.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_references_with_stats(
    index: &FSEIndex,
    query: &QueryRegion,
) -> QueryReferenceReport {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let root_classification = query.classify_bounds(&index.root_node().bounds);

    match root_classification {
        QueryBoundsClassification::Covered => {
            return reference_fully_covered_index(index);
        }
        QueryBoundsClassification::Disjoint => {
            return reference_root_disjoint_query(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path keeps the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let total_records = index.root_node().cardinality;
    let reconstructed_records = traversal_report.stats.retained_candidate_records;

    let mut matches = Vec::with_capacity(result_capacity_hint(reconstructed_records));

    append_retained_reference_matches(
        index,
        query,
        &traversal_report.retained_leaves,
        &mut matches,
    );

    let matched_records = matches.len();

    QueryReferenceReport {
        matches,
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

fn reference_fully_covered_index(index: &FSEIndex) -> QueryReferenceReport {
    let total_leaves = index.leaf_count();
    let total_records = index.root_node().cardinality;
    let mut matches = Vec::with_capacity(result_capacity_hint(total_records));

    for shape in index.leaf_reconstruction_shapes() {
        append_covered_leaf_references(*shape, &mut matches);
    }

    QueryReferenceReport {
        matches,
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

fn reference_root_disjoint_query(index: &FSEIndex) -> QueryReferenceReport {
    QueryReferenceReport {
        matches: Vec::new(),
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

fn append_retained_reference_matches(
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

fn append_covered_leaf_references(
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
    debug_assert_eq!(
        node.dimensions(),
        shape.dimensions,
        "cached leaf dimensionality should match node dimensionality"
    );
    debug_assert_eq!(
        node.residuals.cardinality(),
        shape.cardinality,
        "cached leaf cardinality should match residual cardinality"
    );
    debug_assert_eq!(
        query.dimensions(),
        shape.dimensions,
        "query dimensionality should match retained leaf dimensionality"
    );

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

fn ratio_or_zero(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        0.0
    } else {
        numerator as Scalar / denominator as Scalar
    }
}
