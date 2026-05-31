//! Shared query execution stats construction.
//!
//! This module centralizes execution statistics that are shared by owned-result,
//! reusable-buffer, reference-result, reference-visitor, and count-only query paths.

use crate::query::traversal::QueryTraversalReport;
use crate::storage::FSEIndex;

use super::ratio::ratio_or_zero;
use super::reports::{QueryExecutionStats, RetainedLeafBatchExecutionReport};

/// Builds stats for a root-disjoint query.
///
/// # Runtime Role
///
/// Root-disjoint queries finish after the root metadata classification. They do
/// not retain leaves, reconstruct records, or evaluate exact row predicates.
pub(crate) fn root_disjoint_stats(index: &FSEIndex) -> QueryExecutionStats {
    QueryExecutionStats {
        visited_nodes: 1,
        total_leaves: index.leaf_count(),
        retained_leaves: 0,
        retained_leaf_ratio: 0.0,
        total_records: index.root_node().cardinality,
        reconstructed_records: 0,
        matched_records: 0,
        candidate_ratio: 0.0,
    }
}

/// Builds stats for a root-covered query from retained-leaf execution output.
///
/// # Runtime Role
///
/// Root-covered queries bypass normal traversal because the root bounds prove
/// that every indexed row satisfies the query.
pub(crate) fn root_covered_stats(
    index: &FSEIndex,
    batch_report: &RetainedLeafBatchExecutionReport,
) -> QueryExecutionStats {
    root_covered_stats_with_counts(
        index,
        batch_report.reconstructed_records,
        batch_report.matched_records,
    )
}

/// Builds stats for a root-covered query from exact record counts.
///
/// # Runtime Role
///
/// Count-only, reference-result, and reference-visitor paths can already know the root-covered
/// reconstruction and match counts without using a retained-leaf batch report.
pub(crate) fn root_covered_stats_with_counts(
    index: &FSEIndex,
    reconstructed_records: usize,
    matched_records: usize,
) -> QueryExecutionStats {
    let total_leaves = index.leaf_count();
    let total_records = index.root_node().cardinality;

    QueryExecutionStats {
        visited_nodes: 1,
        total_leaves,
        retained_leaves: total_leaves,
        retained_leaf_ratio: ratio_or_zero(total_leaves, total_leaves),
        total_records,
        reconstructed_records,
        matched_records,
        candidate_ratio: ratio_or_zero(reconstructed_records, total_records),
    }
}

/// Seeds execution stats from traversal output.
///
/// # Runtime Role
///
/// Traversal owns metadata accounting. Retained-leaf execution fills in
/// reconstruction and exact match counts afterward.
pub(crate) fn stats_from_traversal(
    index: &FSEIndex,
    traversal_report: &QueryTraversalReport,
) -> QueryExecutionStats {
    QueryExecutionStats {
        visited_nodes: traversal_report.stats.visited_nodes,
        total_leaves: traversal_report.stats.total_leaves,
        retained_leaves: traversal_report.stats.retained_leaves,
        retained_leaf_ratio: traversal_report.stats.retained_leaf_ratio,
        total_records: index.root_node().cardinality,
        ..QueryExecutionStats::default()
    }
}

/// Builds execution stats from traversal output and exact record counts.
///
/// # Runtime Role
///
/// Reference-result and count-only execution paths do not use the retained-leaf
/// batch report type, but they still share the same traversal-derived metadata
/// accounting as owned-result execution.
pub(crate) fn stats_from_traversal_with_counts(
    index: &FSEIndex,
    traversal_report: &QueryTraversalReport,
    reconstructed_records: usize,
    matched_records: usize,
) -> QueryExecutionStats {
    let total_records = index.root_node().cardinality;

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

/// Applies retained-leaf execution counts to seeded query stats.
pub(crate) fn apply_batch_report_to_stats(
    stats: &mut QueryExecutionStats,
    batch_report: &RetainedLeafBatchExecutionReport,
) {
    stats.reconstructed_records = batch_report.reconstructed_records;
    stats.matched_records = batch_report.matched_records;
    stats.candidate_ratio = ratio_or_zero(stats.reconstructed_records, stats.total_records);
}
