//! Count-only root execution cases.

use crate::storage::FSEIndex;

use super::super::reports::QueryCountReport;
use super::super::stats::{root_covered_stats_with_counts, root_disjoint_stats};

/// Counts every indexed row for a root-covered query.
///
/// # Runtime Role
///
/// If the root bounds are covered by the query, every indexed record is known to
/// match. Count-only execution can return the root cardinality without retained
/// leaf reconstruction or result materialization.
pub(super) fn count_fully_covered_index(index: &FSEIndex) -> QueryCountReport {
    let total_records = index.root_node().cardinality;

    QueryCountReport {
        matched_records: total_records,
        stats: root_covered_stats_with_counts(index, total_records, total_records),
    }
}

/// Returns an empty count report for a root-disjoint query.
///
/// # Runtime Role
///
/// Root-disjoint queries finish after the root metadata classification. They do
/// not retain leaves, reconstruct records, evaluate exact row predicates, or
/// materialize result rows.
pub(super) fn count_root_disjoint_query(index: &FSEIndex) -> QueryCountReport {
    QueryCountReport {
        matched_records: 0,
        stats: root_disjoint_stats(index),
    }
}
