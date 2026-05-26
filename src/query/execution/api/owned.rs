//! Fresh owned-result query execution API.

use crate::math::Vector;
use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::storage::FSEIndex;

use super::super::options::QueryExecutionOptions;
use super::super::reports::QueryExecutionReport;
use super::super::retained::execute_classified_retained_leaves_with_candidate_count;
use super::super::root_coverage::execute_fully_covered_index_with_options;
use super::stats::{apply_batch_report_to_stats, root_disjoint_stats, stats_from_traversal};

/// Executes a query against an FSE index.
///
/// # Runtime Role
///
/// This function composes the complete minimal query pipeline using default
/// execution options:
///
/// 1. Metadata pruning.
/// 2. Deferred reconstruction.
/// 3. Exact point-level predicate evaluation.
///
/// # Formal Reference
///
/// This realizes the staged FSE execution model where `Pi(Q, P_k)` is evaluated
/// before invoking `Phi_k(Delta)`, and `q(x)` is evaluated only for retained
/// candidate partitions.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query(index: &FSEIndex, query: &QueryRegion) -> Vec<Vector> {
    execute_query_with_options(index, query, QueryExecutionOptions::default())
}

/// Executes a query using explicit execution options.
///
/// # Runtime Role
///
/// This function allows callers to choose an execution strategy while preserving
/// exact query semantics.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    options: QueryExecutionOptions,
) -> Vec<Vector> {
    execute_query_with_stats_and_options(index, query, options).results
}

/// Executes a query and returns exact matches with execution statistics.
///
/// # Runtime Role
///
/// This function uses default execution options and provides an instrumented
/// execution path for correctness validation, benchmarking, and future
/// optimization work.
///
/// # Formal Reference
///
/// This preserves the required execution order:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_with_stats(index: &FSEIndex, query: &QueryRegion) -> QueryExecutionReport {
    execute_query_with_stats_and_options(index, query, QueryExecutionOptions::default())
}

/// Executes a query with explicit options and returns exact matches with stats.
///
/// # Runtime Role
///
/// Candidate rows are reconstructed into reusable coordinate buffers and then
/// evaluated immediately. An owned `Vector` is allocated only when the row
/// satisfies the exact query predicate.
///
/// Execution options control how retained leaves are processed after traversal.
///
/// # Formal Reference
///
/// This preserves the required execution order:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_with_stats_and_options(
    index: &FSEIndex,
    query: &QueryRegion,
    options: QueryExecutionOptions,
) -> QueryExecutionReport {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let root_classification = query.classify_bounds(&index.root_node().bounds);

    match root_classification {
        QueryBoundsClassification::Covered => {
            // root coverage means the whole index is already proven in range
            return execute_fully_covered_index_with_options(index, query, options);
        }
        QueryBoundsClassification::Disjoint => {
            // root prune means there is no Stage II or Stage III work
            return execute_root_disjoint_query(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path uses the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let mut stats = stats_from_traversal(index, &traversal_report);

    let batch_report = execute_classified_retained_leaves_with_candidate_count(
        index,
        query,
        &traversal_report.retained_leaves,
        traversal_report.stats.retained_candidate_records,
        options,
    );

    apply_batch_report_to_stats(&mut stats, &batch_report);

    QueryExecutionReport {
        results: batch_report.results,
        stats,
    }
}

/// Builds an execution report for a query disjoint from the root bounds.
///
/// # Runtime Role
///
/// If the query does not intersect the root bounding region, the whole index is
/// eliminated by Stage I metadata pruning. No retained-leaf vector, traversal
/// stack, reconstruction buffer, or result-capacity hint is needed.
///
/// # Formal Reference
///
/// This is the root-level form of `Pi(Q, P_k) = 0`.
fn execute_root_disjoint_query(index: &FSEIndex) -> QueryExecutionReport {
    QueryExecutionReport {
        results: Vec::new(),
        stats: root_disjoint_stats(index),
    }
}
