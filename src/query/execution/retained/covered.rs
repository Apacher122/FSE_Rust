//! Covered retained-leaf materialization.

use crate::storage::{LeafReconstructionShape, PartitionNode};

use super::super::reports::RetainedLeafBatchExecutionReport;

#[cfg(any(test, debug_assertions))]
use super::super::leaf_shape_debug::debug_assert_leaf_reconstruction_shape;

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
    #[cfg(any(test, debug_assertions))]
    debug_assert_leaf_reconstruction_shape(node, shape);

    batch_report.reserve_additional_results(shape.cardinality);

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
        batch_report.push_result_1d(centroid_0 + residual_0[row]);
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
        batch_report.push_result_2d(centroid_0 + residual_0[row], centroid_1 + residual_1[row]);
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

        batch_report.push_result_values(values);
    }
}
