//! Retained-leaf execution dispatch.

use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::FSEIndex;

use super::super::options::{QueryExecutionMode, QueryExecutionOptions};
use super::super::parallel::{
    execute_classified_retained_leaves_parallel_with_candidate_count,
    should_execute_retained_leaves_in_parallel,
};
use super::super::reports::RetainedLeafBatchExecutionReport;
use super::super::serial::execute_classified_retained_leaves_serial_with_candidate_count;

/// Executes classified retained leaves using an already known candidate count.
///
/// # Runtime Role
///
/// Traversal already knows how many records are contained by retained leaves.
/// This helper preserves the result-capacity advantage without requiring a
/// second pass over retained leaves before execution.
pub(crate) fn execute_classified_retained_leaves_with_candidate_count(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    match options.mode {
        QueryExecutionMode::Serial => {
            execute_classified_retained_leaves_serial_with_candidate_count(
                index,
                query,
                retained_leaves,
                candidate_count,
            )
        }
        QueryExecutionMode::Parallel => {
            if should_execute_retained_leaves_in_parallel(options, retained_leaves.len()) {
                execute_classified_retained_leaves_parallel_with_candidate_count(
                    index,
                    query,
                    retained_leaves,
                    candidate_count,
                )
            } else {
                // rayon is not free
                execute_classified_retained_leaves_serial_with_candidate_count(
                    index,
                    query,
                    retained_leaves,
                    candidate_count,
                )
            }
        }
    }
}
