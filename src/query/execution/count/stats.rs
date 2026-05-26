//! Count-only execution stats helpers.

use crate::query::traversal::QueryTraversalReport;
use crate::storage::FSEIndex;

use super::super::ratio::ratio_or_zero;
use super::super::reports::QueryExecutionStats;

/// Builds count-only query stats from traversal output and exact count result.
///
/// # Runtime Role
///
/// Traversal owns metadata accounting and retained candidate counts. Count-only
/// retained-leaf execution contributes only the exact matched record count.
pub(super) fn count_stats_from_traversal(
    index: &FSEIndex,
    traversal_report: &QueryTraversalReport,
    matched_records: usize,
) -> QueryExecutionStats {
    let total_records = index.root_node().cardinality;
    let reconstructed_records = traversal_report.stats.retained_candidate_records;

    QueryExecutionStats {
        visited_nodes: traversal_report.stats.visited_nodes,
        total_leaves: traversal_report.stats.total_leaves,
        retained_leaves: traversal_report.stats.retained_leaves,
        retained_leaf_ratio: traversal_report.stats.retained_leaf_ratio,
        total_records,
        reconstructed_records,
        matched_records,
        candidate_ratio: ratio_or_zero(reconstructed_records, total_records),
    }
}
