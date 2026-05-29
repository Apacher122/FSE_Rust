//! Root-covered query execution.

use crate::math::Vector;
use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::FSEIndex;

use super::options::{QueryExecutionMode, QueryExecutionOptions};
use super::reports::{QueryExecutionReport, RetainedLeafBatchExecutionReport};
use super::retained::{
    append_covered_retained_leaf_results, execute_classified_retained_leaves_with_candidate_count,
};
use super::stats::root_covered_stats;

/// Executes a query that fully contains the root bounding region.
///
/// # Runtime Role
///
/// This is the full-index coverage fast path. It bypasses normal traversal
/// because the root bound already proves every indexed record satisfies the
/// query.
///
/// Serial mode streams all leaf rows directly into one output buffer. Parallel
/// mode still uses the classified retained-leaf execution path so the requested
/// execution strategy is preserved for larger datasets.
pub(crate) fn execute_fully_covered_index_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    options: QueryExecutionOptions,
) -> QueryExecutionReport {
    let total_records = index.root_node().cardinality;

    let batch_report = match options.mode {
        QueryExecutionMode::Serial => execute_fully_covered_index_serial(index),
        QueryExecutionMode::Parallel => {
            let retained_leaves = fully_covered_retained_leaves(index);
            execute_classified_retained_leaves_with_candidate_count(
                index,
                query,
                &retained_leaves,
                total_records,
                options,
            )
        }
    };

    let stats = root_covered_stats(index, &batch_report);

    QueryExecutionReport {
        results: batch_report.results,
        stats,
    }
}

/// Executes a fully covered index using direct serial leaf streaming.
///
/// # Runtime Role
///
/// This function skips traversal-produced retained-leaf vectors entirely and
/// reconstructs every leaf row into one batch result. Leaf reconstruction shapes
/// are read from the index cache so the full-coverage path does not rescan or
/// revalidate the node list.
///
/// # Panics
///
/// Panics when the sum of reconstructed leaf rows does not match root
/// cardinality in debug builds.
pub(crate) fn execute_fully_covered_index_serial(
    index: &FSEIndex,
) -> RetainedLeafBatchExecutionReport {
    let candidate_count = index.root_node().cardinality;
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(candidate_count);

    // full coverage can materialize rows directly
    for shape in index.leaf_reconstruction_shapes() {
        let node = &index.nodes[shape.node_id];
        append_covered_retained_leaf_results(node, *shape, &mut batch_report);
    }

    batch_report.truncate_to_accepted_results();

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "fully covered index reconstruction should match root cardinality"
    );

    batch_report
}

/// Executes a fully covered index into a caller-owned result buffer.
///
/// # Runtime Role
///
/// This is the reusable-buffer variant of the full-index coverage path. It keeps
/// root-covered owned-result queries from allocating a new outer result vector
/// when the caller already has one available. It also keeps reusable inner
/// coordinate buffers alive while the new covered result set is written.
pub(crate) fn execute_fully_covered_index_serial_with_results(
    index: &FSEIndex,
    results: Vec<Vector>,
) -> RetainedLeafBatchExecutionReport {
    let candidate_count = index.root_node().cardinality;
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_result_buffer(candidate_count, results);

    for shape in index.leaf_reconstruction_shapes() {
        let node = &index.nodes[shape.node_id];
        append_covered_retained_leaf_results(node, *shape, &mut batch_report);
    }

    batch_report.truncate_to_accepted_results();

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "fully covered index reconstruction should match root cardinality"
    );

    batch_report
}

/// Returns retained-leaf records for every leaf in the index.
///
/// # Runtime Role
///
/// Parallel fully covered queries still need retained-leaf work units. Every
/// leaf is classified as covered because root containment proves all descendants
/// are covered.
pub(crate) fn fully_covered_retained_leaves(index: &FSEIndex) -> Vec<RetainedLeaf> {
    let mut retained_leaves = Vec::with_capacity(index.leaf_count());

    for shape in index.leaf_reconstruction_shapes() {
        retained_leaves.push(RetainedLeaf::covered_with_shape(*shape));
    }

    retained_leaves
}

/// Returns the number of leaf partitions in an index.
///
/// # Runtime Role
///
/// This compatibility helper is kept for execution root-coverage tests that
/// import `leaf_count` from `crate::query::execution`. Runtime code should use
/// `FSEIndex::leaf_count()` directly.
#[cfg(test)]
pub(crate) fn leaf_count(index: &FSEIndex) -> usize {
    index.leaf_count()
}
