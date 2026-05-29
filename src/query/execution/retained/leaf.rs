//! Retained-leaf row execution orchestration.

use crate::math::Scalar;
use crate::query::{QueryRegion, RetainedLeafCoverage};
use crate::storage::{LeafReconstructionShape, PartitionNode};

use super::super::reports::{RetainedLeafBatchExecutionReport, RetainedLeafExecutionReport};

pub(crate) use super::covered::append_covered_retained_leaf_results;
pub(crate) use super::partial::append_partially_covered_retained_leaf_results;

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

    batch_report.truncate_to_accepted_results();

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
