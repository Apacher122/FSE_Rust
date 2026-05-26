//! Reference-result query execution.
//!
//! This module provides an exact query output contract that returns references
//! to matching residual rows instead of materializing owned `Vector` values.

use crate::math::{Scalar, Vector};
use crate::query::reconstruction::{reconstruct_point, reconstruct_row_into};
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::reports::{
    QueryExecutionStats, QueryReferenceReport, QueryResultReference, result_capacity_hint,
};

/// Reconstructs an owned row from an exact query result reference.
///
/// # Runtime Role
///
/// Reference-result queries return stable leaf/row identifiers instead of
/// materializing owned [`Vector`] values. This helper is the explicit
/// reconstruction seam for callers that later need one referenced row as owned
/// coordinates.
///
/// # Formal Reference
///
/// This applies the reconstruction operator `Phi_k(Delta) = mu_k + Delta` to
/// the referenced residual row. It does not rerun geometric pruning or exact
/// predicate evaluation.
///
/// # Panics
///
/// Panics when the referenced node does not exist, does not identify a leaf
/// partition, or the referenced row is outside that leaf's residual storage.
pub fn reconstruct_query_result_reference(
    index: &FSEIndex,
    reference: QueryResultReference,
) -> Vector {
    let node = reference_leaf_node(index, reference);

    reconstruct_point(node, reference.row_index)
}

/// Reconstructs an exact query result reference into a caller-owned coordinate buffer.
///
/// # Runtime Role
///
/// This is the allocation-conscious reconstruction path for reference-result
/// rows. The caller owns `output` and may reuse it across many references.
///
/// # Formal Reference
///
/// This applies `Phi_k(Delta) = mu_k + Delta` to the referenced residual row
/// while preserving the same row-level reconstruction semantics as owned-result
/// query execution.
///
/// # Panics
///
/// Panics under the same reference validation rules as
/// [`reconstruct_query_result_reference`].
pub fn reconstruct_query_result_reference_into(
    index: &FSEIndex,
    reference: QueryResultReference,
    output: &mut Vec<Scalar>,
) {
    let node = reference_leaf_node(index, reference);

    reconstruct_row_into(node, reference.row_index, output);
}

/// Reconstructs owned rows from exact query result references.
///
/// # Runtime Role
///
/// This helper reconstructs a reference-result batch only when a caller chooses
/// to materialize the referenced rows. Query execution can still return
/// references without paying owned `Vector` materialization cost up front.
///
/// # Formal Reference
///
/// Each reference is reconstructed by applying `Phi_k(Delta) = mu_k + Delta` to
/// its referenced residual row.
///
/// # Panics
///
/// Panics when any reference is invalid under the same rules as
/// [`reconstruct_query_result_reference`].
pub fn reconstruct_query_result_references(
    index: &FSEIndex,
    references: &[QueryResultReference],
) -> Vec<Vector> {
    let mut results = Vec::with_capacity(references.len());

    reconstruct_query_result_references_into(index, references, &mut results);

    results
}

/// Reconstructs exact query result references into a caller-owned result buffer.
///
/// # Runtime Role
///
/// This is the batch reconstruction equivalent of `execute_query_into` for
/// reference results. The caller owns the output vector and may reuse it across
/// repeated reconstruction calls. Existing result slots are overwritten in
/// place where possible so their inner coordinate buffers can be reused.
///
/// # Formal Reference
///
/// This function performs only deferred reconstruction. It does not rerun
/// geometric pruning or exact predicate evaluation because the supplied
/// references already represent accepted rows.
///
/// # Panics
///
/// Panics when any supplied reference is invalid.
pub fn reconstruct_query_result_references_into(
    index: &FSEIndex,
    references: &[QueryResultReference],
    results: &mut Vec<Vector>,
) {
    let target_capacity = references.len();

    if results.capacity() < target_capacity {
        results.reserve_exact(target_capacity - results.capacity());
    }

    let mut result_len = 0;

    for reference in references {
        reconstruct_reference_into_result_slot(index, *reference, results, result_len);
        result_len += 1;
    }

    results.truncate(result_len);
}

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

fn reconstruct_reference_into_result_slot(
    index: &FSEIndex,
    reference: QueryResultReference,
    results: &mut Vec<Vector>,
    slot_index: usize,
) {
    let node = reference_leaf_node(index, reference);

    if slot_index < results.len() {
        reconstruct_row_into(node, reference.row_index, &mut results[slot_index].values);
    } else {
        results.push(reconstruct_point(node, reference.row_index));
    }
}

fn reference_leaf_node(index: &FSEIndex, reference: QueryResultReference) -> &PartitionNode {
    let node = index.nodes.get(reference.node_id).unwrap_or_else(|| {
        panic!(
            "query result reference node id {} is outside the index",
            reference.node_id
        )
    });

    assert!(
        node.is_leaf,
        "query result reference node id {} must reference a leaf partition",
        reference.node_id
    );

    node
}

fn ratio_or_zero(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        0.0
    } else {
        numerator as Scalar / denominator as Scalar
    }
}
