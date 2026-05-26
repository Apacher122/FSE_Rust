//! Diagnostic retained-leaf execution API.

use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::FSEIndex;

use super::super::options::QueryExecutionOptions;
use super::super::reports::RetainedLeafBatchExecutionReport;
use super::super::retained::execute_classified_retained_leaves_with_candidate_count;

/// Executes a pre-classified retained-leaf batch for benchmark diagnostics.
///
/// # Runtime Role
///
/// This helper isolates the retained-leaf execution portion of the query
/// pipeline after traversal has already selected candidate leaves. It is used by
/// benchmark debug output to estimate Stage II and Stage III cost without
/// changing the public query API or normal execution path.
///
/// # Formal Reference
///
/// This corresponds to the post-selection work in the staged FSE execution
/// model: deferred reconstruction followed by exact predicate evaluation.
pub(crate) fn execute_retained_leaf_batch_for_diagnostics(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    execute_classified_retained_leaves_with_candidate_count(
        index,
        query,
        retained_leaves,
        candidate_count,
        options,
    )
}
