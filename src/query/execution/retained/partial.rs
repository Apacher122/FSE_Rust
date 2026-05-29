//! Partially covered retained-leaf exact matching.

use crate::math::Scalar;
use crate::query::QueryRegion;
use crate::query::reconstruction::reconstruct_row_into_prevalidated;
use crate::storage::{LeafReconstructionShape, PartitionNode};

use super::super::reports::RetainedLeafBatchExecutionReport;

#[cfg(any(test, debug_assertions))]
use super::super::leaf_shape_debug::{
    debug_assert_leaf_reconstruction_shape, debug_assert_query_reconstruction_shape,
};

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
    #[cfg(any(test, debug_assertions))]
    {
        debug_assert_leaf_reconstruction_shape(node, shape);
        debug_assert_query_reconstruction_shape(query, shape);
    }

    let original_match_count = batch_report.accepted_result_count();

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

    let matched_records = batch_report.accepted_result_count() - original_match_count;

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
            batch_report.push_result_1d(value_0);
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
            batch_report.push_result_2d(value_0, value_1);
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
            batch_report.push_result_from_buffer(reconstructed_values, shape.dimensions);
        }
    }
}
