//! Parallel retained-leaf execution.

use rayon::prelude::*;

use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::FSEIndex;

use super::options::{QueryExecutionMode, QueryExecutionOptions};
use super::reports::{RetainedLeafBatchExecutionReport, RetainedLeafExecutionReport};
use super::retained::{
    execute_retained_leaf_with_cached_shape, merge_retained_leaf_reports_in_order,
};

#[cfg(test)]
use super::retained::classified_retained_candidate_count;
#[cfg(any(test, debug_assertions))]
use super::retained::validate_retained_leaves;

/// Returns true when parallel mode should use Rayon for the retained-leaf batch.
///
/// # Runtime Role
///
/// This policy prevents small retained-leaf batches from paying parallel
/// scheduling overhead. Serial mode always returns false.
pub(crate) fn should_execute_retained_leaves_in_parallel(
    options: QueryExecutionOptions,
    retained_leaf_count: usize,
) -> bool {
    matches!(options.mode, QueryExecutionMode::Parallel)
        && retained_leaf_count >= options.parallel_min_retained_leaves
}

/// Executes classified retained leaves using Rayon-backed parallel iteration.
///
/// # Runtime Role
///
/// This compatibility helper is test-only. The release path should pass a
/// candidate count from traversal and call
/// `execute_classified_retained_leaves_parallel_with_candidate_count`.
#[cfg(test)]
pub(crate) fn execute_classified_retained_leaves_parallel(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
) -> RetainedLeafBatchExecutionReport {
    let candidate_count = classified_retained_candidate_count(index, retained_leaves);

    execute_classified_retained_leaves_parallel_with_candidate_count(
        index,
        query,
        retained_leaves,
        candidate_count,
    )
}

/// Executes classified retained leaves in parallel with a known candidate count.
///
/// # Runtime Role
///
/// Parallel execution still needs leaf-local buffers, but it does not need to
/// recount retained candidates when traversal already supplied that count.
pub(crate) fn execute_classified_retained_leaves_parallel_with_candidate_count(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
) -> RetainedLeafBatchExecutionReport {
    #[cfg(any(test, debug_assertions))]
    validate_retained_leaves(index, retained_leaves);

    // rayon collect preserves order for this indexed slice iterator
    let leaf_reports: Vec<RetainedLeafExecutionReport> = retained_leaves
        .par_iter()
        .map(|retained_leaf| {
            let node = &index.nodes[retained_leaf.node_id];
            let shape = index.leaf_reconstruction_shape(retained_leaf.node_id);

            // parallel still needs leaf local buffers
            execute_retained_leaf_with_cached_shape(node, query, shape, retained_leaf.coverage)
        })
        .collect();

    let batch_report = merge_retained_leaf_reports_in_order(leaf_reports, candidate_count);

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "retained candidate count should match reconstructed retained rows"
    );

    batch_report
}
