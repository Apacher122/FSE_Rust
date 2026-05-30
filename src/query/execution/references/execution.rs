//! Reference-result query execution path.

use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::storage::FSEIndex;

use super::super::reports::{QueryReferenceReport, QueryResultReference, result_capacity_hint};
use super::super::root::classify_query_root;
use super::super::stats::{
    root_covered_stats_with_counts, root_disjoint_stats, stats_from_traversal_with_counts,
};
use super::matching::{append_fully_covered_index_references, append_retained_reference_matches};

/// Executes a query and returns exact matching row references.
///
/// # Runtime Role
///
/// This function preserves the same pruning and exact predicate semantics as
/// owned-result query execution, but returns leaf/row references instead of
/// allocating one owned vector per match.
///
/// # Formal Reference
///
/// This preserves the required execution order:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// The reconstruction step is still used for exact predicate evaluation on
/// partially covered leaves. The returned references identify members of
/// `E(Q, F)` without materializing owned `Vector` values.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_references(
    index: &FSEIndex,
    query: &QueryRegion,
) -> Vec<QueryResultReference> {
    execute_query_references_with_stats(index, query).matches
}

/// Executes a query and returns exact matching row references with stats.
///
/// # Runtime Role
///
/// `QueryReferenceReport` exposes a lower-allocation representation of the
/// exact result set `E(Q, F)`.
///
/// The public query APIs use distinct output contracts over the same execution
/// semantics:
///
/// - owned-result execution materializes owned `Vector` values.
/// - reference-result execution materializes row references.
/// - count-only execution returns exact cardinality.
/// - existence execution returns whether the exact result set is non-empty.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_references_with_stats(
    index: &FSEIndex,
    query: &QueryRegion,
) -> QueryReferenceReport {
    let root_classification = classify_query_root(index, query);

    match root_classification {
        QueryBoundsClassification::Covered => {
            return reference_fully_covered_index(index);
        }
        QueryBoundsClassification::Disjoint => {
            return reference_root_disjoint_query(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path keeps the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let reconstructed_records = traversal_report.stats.retained_candidate_records;

    let mut matches = Vec::with_capacity(result_capacity_hint(reconstructed_records));

    append_retained_reference_matches(
        index,
        query,
        &traversal_report.retained_leaves,
        &mut matches,
    );

    let matched_records = matches.len();

    QueryReferenceReport {
        matches,
        stats: stats_from_traversal_with_counts(
            index,
            &traversal_report,
            reconstructed_records,
            matched_records,
        ),
    }
}

fn reference_fully_covered_index(index: &FSEIndex) -> QueryReferenceReport {
    let total_records = index.root_node().cardinality;
    let mut matches = Vec::with_capacity(result_capacity_hint(total_records));

    append_fully_covered_index_references(index, &mut matches);

    QueryReferenceReport {
        matches,
        stats: root_covered_stats_with_counts(index, total_records, total_records),
    }
}

fn reference_root_disjoint_query(index: &FSEIndex) -> QueryReferenceReport {
    QueryReferenceReport {
        matches: Vec::new(),
        stats: root_disjoint_stats(index),
    }
}
