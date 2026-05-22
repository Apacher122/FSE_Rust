//! Retained-leaf row execution.

use crate::math::{Scalar, Vector};
use crate::query::reconstruction::reconstruct_row_into_prevalidated;
use crate::query::{QueryRegion, RetainedLeafCoverage};
use crate::storage::{LeafReconstructionShape, PartitionNode};

use super::super::reports::{RetainedLeafBatchExecutionReport, RetainedLeafExecutionReport};
use super::merge::reserve_additional_results;

/// Executes one retained leaf using cached reconstruction shape metadata.
///
/// # Runtime Role
///
/// This is the leaf-local execution path used by parallel retained-leaf
/// execution. The index already validated and cached the leaf shape, so this
/// function avoids per-query shape validation.
///
/// Traversal validation already guarantees that retained work units reference
/// leaf nodes. The leaf check remains debug-only so development builds still
/// catch invariant violations without keeping the duplicate guard in release
/// timing.
pub(crate) fn execute_retained_leaf_with_cached_shape(
    node: &PartitionNode,
    query: &QueryRegion,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
) -> RetainedLeafExecutionReport {
    debug_assert!(
        node.is_leaf,
        "retained leaf execution helper requires a leaf node"
    );

    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(shape.cardinality);

    match coverage {
        RetainedLeafCoverage::Covered => {
            append_covered_retained_leaf_results(node, shape, &mut batch_report);
        }
        RetainedLeafCoverage::Partial => {
            let mut reconstructed_values = Vec::with_capacity(shape.dimensions);
            append_partially_covered_retained_leaf_results(
                node,
                shape,
                query,
                &mut batch_report,
                &mut reconstructed_values,
            );
        }
    }

    RetainedLeafExecutionReport {
        results: batch_report.results,
        reconstructed_records: batch_report.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records: batch_report.predicate_evaluated_records,
        matched_records: batch_report.matched_records,
    }
}

/// Streams one retained leaf into an existing batch report.
///
/// # Runtime Role
///
/// This is the serial execution hot path. It preserves retained-leaf ordering
/// while avoiding a temporary result vector and merge step for each leaf.
///
/// Traversal validation already guarantees that retained entries reference leaf
/// nodes. The leaf check remains debug-only because release execution should not
/// pay for the same invariant twice.
pub(crate) fn execute_retained_leaf_into_batch_report(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    coverage: RetainedLeafCoverage,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    debug_assert!(
        node.is_leaf,
        "retained leaf streaming helper requires a leaf node"
    );

    match coverage {
        RetainedLeafCoverage::Covered => {
            append_covered_retained_leaf_results(node, shape, batch_report)
        }
        RetainedLeafCoverage::Partial => append_partially_covered_retained_leaf_results(
            node,
            shape,
            query,
            batch_report,
            reconstructed_values,
        ),
    }
}

/// Appends all rows from a covered retained leaf into the batch report.
///
/// # Runtime Role
///
/// The query already contains the leaf bounds, so every reconstructed row can be
/// appended directly without exact predicate evaluation. The dimensional branch
/// is hoisted once per leaf so the row loop does not repeatedly dispatch through
/// the generic reconstruction helper.
pub(crate) fn append_covered_retained_leaf_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    batch_report: &mut RetainedLeafBatchExecutionReport,
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

    reserve_additional_results(&mut batch_report.results, shape.cardinality);

    match shape.dimensions {
        1 => append_covered_1d_results(node, shape, batch_report),
        2 => append_covered_2d_results(node, shape, batch_report),
        _ => append_covered_generic_results(node, shape, batch_report),
    }

    batch_report.reconstructed_records += shape.cardinality;
    batch_report.matched_records += shape.cardinality;
}

#[inline]
fn append_covered_1d_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    batch_report: &mut RetainedLeafBatchExecutionReport,
) {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    // covered geometry means every row is already accepted
    for row in 0..shape.cardinality {
        batch_report
            .results
            .push(Vector::new(vec![centroid_0 + residual_0[row]]));
    }
}

#[inline]
fn append_covered_2d_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    batch_report: &mut RetainedLeafBatchExecutionReport,
) {
    let centroid_0 = node.centroid[0];
    let centroid_1 = node.centroid[1];

    let residual_0 = &node.residuals.dimensions[0];
    let residual_1 = &node.residuals.dimensions[1];

    // same exact reconstruction, just with the shape branch outside the row loop
    for row in 0..shape.cardinality {
        batch_report.results.push(Vector::new(vec![
            centroid_0 + residual_0[row],
            centroid_1 + residual_1[row],
        ]));
    }
}

#[inline]
fn append_covered_generic_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    batch_report: &mut RetainedLeafBatchExecutionReport,
) {
    for row in 0..shape.cardinality {
        let mut values = Vec::with_capacity(shape.dimensions);

        for (centroid_value, residual_dimension) in
            node.centroid.iter().zip(&node.residuals.dimensions)
        {
            values.push(*centroid_value + residual_dimension[row]);
        }

        batch_report.results.push(Vector::new(values));
    }
}

/// Appends matching rows from a partially covered retained leaf into the batch report.
///
/// # Runtime Role
///
/// Partial leaves still use exact predicate semantics. For the common 1D and 2D
/// paths, reconstruction and predicate checks are fused so non-matching rows do
/// not allocate temporary coordinate vectors. Higher-dimensional queries keep
/// the generic scratch-buffer path.
pub(crate) fn append_partially_covered_retained_leaf_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
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

    let original_match_count = batch_report.results.len();

    match shape.dimensions {
        1 => append_partially_covered_1d_results(node, shape, query, batch_report),
        2 => append_partially_covered_2d_results(node, shape, query, batch_report),
        _ => append_partially_covered_generic_results(
            node,
            shape,
            query,
            batch_report,
            reconstructed_values,
        ),
    }

    let matched_records = batch_report.results.len() - original_match_count;

    batch_report.reconstructed_records += shape.cardinality;
    #[cfg(test)]
    {
        batch_report.predicate_evaluated_records += shape.cardinality;
    }
    batch_report.matched_records += matched_records;
}

#[inline]
fn append_partially_covered_1d_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    batch_report: &mut RetainedLeafBatchExecutionReport,
) {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];

    // dont allocate a row vec unless this row actually matches
    for row in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row];

        if value_0 >= query_min_0 && value_0 <= query_max_0 {
            batch_report.results.push(Vector::new(vec![value_0]));
        }
    }
}

#[inline]
fn append_partially_covered_2d_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    batch_report: &mut RetainedLeafBatchExecutionReport,
) {
    let centroid_0 = node.centroid[0];
    let centroid_1 = node.centroid[1];

    let residual_0 = &node.residuals.dimensions[0];
    let residual_1 = &node.residuals.dimensions[1];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];
    let query_min_1 = query.min[1];
    let query_max_1 = query.max[1];

    for row in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row];

        if value_0 < query_min_0 || value_0 > query_max_0 {
            continue;
        }

        let value_1 = centroid_1 + residual_1[row];

        if value_1 >= query_min_1 && value_1 <= query_max_1 {
            batch_report
                .results
                .push(Vector::new(vec![value_0, value_1]));
        }
    }
}

#[inline]
fn append_partially_covered_generic_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    // keep the generic buffered reconstruction path for higher-dimensional data
    for row in 0..shape.cardinality {
        reconstruct_row_into_prevalidated(node, row, shape.dimensions, reconstructed_values);

        if query.contains_values_prevalidated(reconstructed_values, shape.dimensions) {
            push_reconstructed_values_as_result(
                &mut batch_report.results,
                reconstructed_values,
                shape.dimensions,
            );
        }
    }
}

/// Moves a reconstructed row buffer into the final result set.
///
/// # Runtime Role
///
/// Matching rows must still become owned `Vector` values because that is the
/// query API contract. Moving the scratch buffer avoids cloning the same row
/// after exact predicate evaluation has already accepted it.
fn push_reconstructed_values_as_result(
    results: &mut Vec<Vector>,
    reconstructed_values: &mut Vec<Scalar>,
    dimensions: usize,
) {
    debug_assert_eq!(
        reconstructed_values.len(),
        dimensions,
        "reconstructed row dimensionality should match the partition"
    );

    let accepted_values = std::mem::replace(reconstructed_values, Vec::with_capacity(dimensions));

    results.push(Vector::new(accepted_values));
}
