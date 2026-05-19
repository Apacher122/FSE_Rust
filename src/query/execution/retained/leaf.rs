//! Retained-leaf row execution.

use crate::math::{Scalar, Vector};
use crate::query::reconstruction::{
    reconstruct_point_prevalidated, reconstruct_row_into_prevalidated,
};
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
pub(crate) fn execute_retained_leaf_with_cached_shape(
    node: &PartitionNode,
    query: &QueryRegion,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
) -> RetainedLeafExecutionReport {
    assert!(
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
pub(crate) fn execute_retained_leaf_into_batch_report(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    coverage: RetainedLeafCoverage,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    assert!(
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
/// appended directly without exact predicate evaluation.
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

    // geometry already proved these rows match
    for row in 0..shape.cardinality {
        batch_report
            .results
            .push(reconstruct_point_prevalidated(node, row, shape.dimensions));
    }

    batch_report.reconstructed_records += shape.cardinality;
    batch_report.matched_records += shape.cardinality;
}

/// Appends matching rows from a partially covered retained leaf into the batch report.
///
/// # Runtime Role
///
/// Partial leaves still use the exact predicate path. The retained-leaf shape is
/// read from the index cache before the row loop, then each candidate row is
/// reconstructed into the reusable scratch buffer before exact query evaluation.
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

    // keep the restored buffered reconstruction path
    // only the exact predicate check uses the prevalidated hot helper
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

    let matched_records = batch_report.results.len() - original_match_count;

    batch_report.reconstructed_records += shape.cardinality;
    #[cfg(test)]
    {
        batch_report.predicate_evaluated_records += shape.cardinality;
    }
    batch_report.matched_records += matched_records;
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
