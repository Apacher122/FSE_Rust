//! Root-covered query execution.

use crate::math::Scalar;
use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::FSEIndex;

use super::options::{QueryExecutionMode, QueryExecutionOptions};
use super::reports::{QueryExecutionReport, QueryExecutionStats, RetainedLeafBatchExecutionReport};
use super::retained::{
    append_covered_retained_leaf_results, execute_classified_retained_leaves_with_candidate_count,
};

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
    let total_leaves = index.leaf_count();
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

    let stats = QueryExecutionStats {
        visited_nodes: 1,
        total_leaves,
        retained_leaves: total_leaves,
        retained_leaf_ratio: if total_leaves == 0 { 0.0 } else { 1.0 },
        total_records,
        reconstructed_records: batch_report.reconstructed_records,
        matched_records: batch_report.matched_records,
        candidate_ratio: if total_records == 0 {
            0.0
        } else {
            batch_report.reconstructed_records as Scalar / total_records as Scalar
        },
    };

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
/// reconstructs every leaf row into one batch result.
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
    for node in index.nodes.iter().filter(|node| node.is_leaf) {
        append_covered_retained_leaf_results(node, &mut batch_report);
    }

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

    for (node_id, node) in index.nodes.iter().enumerate() {
        if node.is_leaf {
            retained_leaves.push(RetainedLeaf::covered(node_id));
        }
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
