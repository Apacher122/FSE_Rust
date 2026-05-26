//! Serial retained-leaf execution.

use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::FSEIndex;

use super::reports::RetainedLeafBatchExecutionReport;
use super::retained::execute_retained_leaf_into_batch_report;

#[cfg(test)]
use super::retained::classified_retained_candidate_count;
#[cfg(any(test, debug_assertions))]
use super::retained::validate_retained_leaves;

/// Executes classified retained leaves using deterministic serial iteration.
///
/// # Runtime Role
///
/// This compatibility helper is test-only. The release path should pass a
/// candidate count from traversal and call
/// `execute_classified_retained_leaves_serial_with_candidate_count`.
#[cfg(test)]
pub(crate) fn execute_classified_retained_leaves_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
) -> RetainedLeafBatchExecutionReport {
    let candidate_count = classified_retained_candidate_count(index, retained_leaves);

    execute_classified_retained_leaves_serial_with_candidate_count(
        index,
        query,
        retained_leaves,
        candidate_count,
    )
}

/// Executes classified retained leaves serially with a known candidate count.
///
/// # Runtime Role
///
/// The release query path reaches this through traversal, which already counted
/// retained records. That avoids a recount while preserving exact result-buffer
/// capacity planning.
pub(crate) fn execute_classified_retained_leaves_serial_with_candidate_count(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
) -> RetainedLeafBatchExecutionReport {
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(candidate_count);

    execute_classified_retained_leaves_serial_into_batch_report(
        index,
        query,
        retained_leaves,
        candidate_count,
        &mut batch_report,
    );

    batch_report.truncate_to_accepted_results();

    batch_report
}

/// Executes classified retained leaves serially into a caller-provided result buffer.
///
/// # Runtime Role
///
/// This variant preserves the same serial retained-leaf semantics as
/// `execute_classified_retained_leaves_serial_with_candidate_count`, but starts
/// from an existing `Vec<Vector>` allocation. It is used by `execute_query_into`
/// to reuse both the outer result buffer and previously allocated inner
/// coordinate buffers when repeated queries return compatible result rows.
pub(crate) fn execute_classified_retained_leaves_serial_with_candidate_count_and_results(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
    results: Vec<crate::math::Vector>,
) -> RetainedLeafBatchExecutionReport {
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_result_buffer(candidate_count, results);

    execute_classified_retained_leaves_serial_into_batch_report(
        index,
        query,
        retained_leaves,
        candidate_count,
        &mut batch_report,
    );

    batch_report.truncate_to_accepted_results();

    batch_report
}

fn execute_classified_retained_leaves_serial_into_batch_report(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
    batch_report: &mut RetainedLeafBatchExecutionReport,
) {
    #[cfg(any(test, debug_assertions))]
    validate_retained_leaves(index, retained_leaves);

    // 1d and 2d retained-leaf paths do not need this buffer
    // let the generic path allocate it lazily only if it actually runs
    let mut reconstructed_values = Vec::new();

    for retained_leaf in retained_leaves {
        let node = &index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(index);

        execute_retained_leaf_into_batch_report(
            node,
            shape,
            query,
            retained_leaf.coverage,
            batch_report,
            &mut reconstructed_values,
        );
    }

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "retained candidate count should match reconstructed retained rows"
    );
}
