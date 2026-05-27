//! Public count-only query execution API.

use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::storage::FSEIndex;

use super::super::reports::QueryCountReport;
use super::super::root::classify_query_root;
use super::execution::{
    count_fully_covered_index, count_retained_matches_without_results, count_root_disjoint_query,
};
use super::stats::count_stats_from_traversal;

/// Counts exact query matches without materializing owned result vectors.
///
/// # Runtime Role
///
/// This function is useful when a caller needs cardinality, existence checks,
/// or aggregate planning information without paying the owned-result
/// materialization cost of `execute_query`.
///
/// # Formal Reference
///
/// This preserves the staged execution model:
///
/// `Geometry -> Reconstruction -> Logic`
///
/// The difference from owned-result execution is that accepted rows increment a
/// counter instead of being converted into returned `Vector` values.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn count_query_matches(index: &FSEIndex, query: &QueryRegion) -> usize {
    count_query_matches_with_stats(index, query).matched_records
}

/// Counts exact query matches and returns execution statistics.
///
/// # Runtime Role
///
/// This function exposes the same structural work counters used by owned-result
/// query execution while avoiding final result allocation. `matched_records` is
/// duplicated at the top level for convenience and inside `stats` for
/// consistency with the existing execution report shape.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn count_query_matches_with_stats(index: &FSEIndex, query: &QueryRegion) -> QueryCountReport {
    let root_classification = classify_query_root(index, query);

    match root_classification {
        QueryBoundsClassification::Covered => {
            return count_fully_covered_index(index);
        }
        QueryBoundsClassification::Disjoint => {
            return count_root_disjoint_query(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path uses the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let matched_records =
        count_retained_matches_without_results(index, query, &traversal_report.retained_leaves);

    QueryCountReport {
        matched_records,
        stats: count_stats_from_traversal(index, &traversal_report, matched_records),
    }
}
