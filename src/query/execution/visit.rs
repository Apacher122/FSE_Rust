//! Exact query reference visitor execution.
//!
//! Visitor execution streams exact matching row references to a caller-provided
//! function. It preserves the same staged execution semantics as the owned,
//! reference, count-only, and existence output contracts without allocating a
//! result vector for the references themselves.

use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::storage::FSEIndex;

use super::references::matching::{
    for_each_fully_covered_index_reference, for_each_retained_reference_match,
};
use super::references::{QueryResultRowView, query_result_row_view};
use super::reports::{QueryExecutionStats, QueryResultReference};
use super::root::classify_query_root;
use super::stats::{
    root_covered_stats_with_counts, root_disjoint_stats, stats_from_traversal_with_counts,
};

/// Visits exact matching row references and returns execution statistics.
///
/// # Runtime Role
///
/// This output contract is useful when a caller wants to stream exact matches
/// into an external accumulator, callback, or row-processing pipeline without
/// allocating a `Vec<QueryResultReference>`.
///
/// # Formal Reference
///
/// This preserves the staged execution model:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// The visitor receives references to members of the exact result set
/// `E(Q, F) = σ_Q(Φ(R_T(Q)))`. Partial retained leaves still reconstruct
/// candidate coordinates before exact predicate evaluation.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn visit_query_references<F>(
    index: &FSEIndex,
    query: &QueryRegion,
    mut visitor: F,
) -> QueryExecutionStats
where
    F: FnMut(QueryResultReference),
{
    let root_classification = classify_query_root(index, query);

    match root_classification {
        QueryBoundsClassification::Covered => {
            return visit_fully_covered_index_references(index, visitor);
        }
        QueryBoundsClassification::Disjoint => {
            return root_disjoint_stats(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path uses the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let reconstructed_records = traversal_report.stats.retained_candidate_records;
    let mut matched_records = 0;

    for_each_retained_reference_match(
        index,
        query,
        &traversal_report.retained_leaves,
        |reference| {
            matched_records += 1;
            visitor(reference);
        },
    );

    stats_from_traversal_with_counts(
        index,
        &traversal_report,
        reconstructed_records,
        matched_records,
    )
}

/// Visits exact matching rows as borrowed row views and returns execution statistics.
///
/// # Runtime Role
///
/// This output contract streams exact matches without allocating owned
/// [`crate::math::Vector`] values or a `Vec<QueryResultReference>`. The caller
/// receives a [`QueryResultRowView`] for each exact match and chooses whether to
/// inspect coordinates lazily, write them into a reusable buffer, or materialize
/// an owned vector.
///
/// # Formal Reference
///
/// This preserves the staged execution model:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// The row view exposes deferred reconstruction through
/// $\Phi_k(\Delta) = \mu_k + \Delta$ after exact predicate evaluation has
/// accepted the referenced row.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn visit_query_row_views<'a, F>(
    index: &'a FSEIndex,
    query: &QueryRegion,
    mut visitor: F,
) -> QueryExecutionStats
where
    F: FnMut(QueryResultRowView<'a>),
{
    visit_query_references(index, query, |reference| {
        visitor(query_result_row_view(index, reference));
    })
}

fn visit_fully_covered_index_references<F>(index: &FSEIndex, mut visitor: F) -> QueryExecutionStats
where
    F: FnMut(QueryResultReference),
{
    let total_records = index.root_node().cardinality;
    let mut matched_records = 0;

    for_each_fully_covered_index_reference(index, |reference| {
        matched_records += 1;
        visitor(reference);
    });

    root_covered_stats_with_counts(index, total_records, matched_records)
}
