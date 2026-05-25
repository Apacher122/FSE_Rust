//! Public query execution API.

use crate::math::{Scalar, Vector};
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::FSEIndex;

use super::options::{QueryExecutionMode, QueryExecutionOptions};
use super::reports::{QueryExecutionReport, QueryExecutionStats, RetainedLeafBatchExecutionReport};
use super::retained::execute_classified_retained_leaves_with_candidate_count;
use super::root_coverage::{
    execute_fully_covered_index_serial_with_results, execute_fully_covered_index_with_options,
};
use super::serial::execute_classified_retained_leaves_serial_with_candidate_count_and_results;

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

/// Executes a query into a caller-provided result buffer.
///
/// # Runtime Role
///
/// This is the reusable-buffer owned-result API. It preserves the same exact
/// query semantics as [`execute_query`], but writes the results into `results`
/// instead of returning a freshly allocated outer `Vec<Vector>`.
///
/// The buffer is cleared before use. Existing capacity may be reused.
///
/// # Formal Reference
///
/// This still follows the staged FSE execution model:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_into(
    index: &FSEIndex,
    query: &QueryRegion,
    results: &mut Vec<Vector>,
) -> QueryExecutionStats {
    execute_query_into_with_options(index, query, QueryExecutionOptions::serial(), results)
}

/// Executes a query into a caller-provided result buffer using explicit options.
///
/// # Runtime Role
///
/// Serial execution reuses the caller-provided result buffer directly. Parallel
/// execution preserves exact semantics but currently falls back to the existing
/// owned-result report path before replacing the caller buffer.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_into_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    options: QueryExecutionOptions,
    results: &mut Vec<Vector>,
) -> QueryExecutionStats {
    match options.mode {
        QueryExecutionMode::Serial => execute_query_into_serial(index, query, results),
        QueryExecutionMode::Parallel => {
            let report = execute_query_with_stats_and_options(index, query, options);
            results.clear();
            results.extend(report.results);
            report.stats
        }
    }
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

    let mut stats = QueryExecutionStats {
        visited_nodes: traversal_report.stats.visited_nodes,
        total_leaves: traversal_report.stats.total_leaves,
        retained_leaves: traversal_report.stats.retained_leaves,
        retained_leaf_ratio: traversal_report.stats.retained_leaf_ratio,
        total_records: index.root_node().cardinality,
        ..QueryExecutionStats::default()
    };

    let batch_report = execute_classified_retained_leaves_with_candidate_count(
        index,
        query,
        &traversal_report.retained_leaves,
        traversal_report.stats.retained_candidate_records,
        options,
    );

    stats.reconstructed_records = batch_report.reconstructed_records;
    stats.matched_records = batch_report.matched_records;

    stats.candidate_ratio = if stats.total_records == 0 {
        0.0
    } else {
        stats.reconstructed_records as Scalar / stats.total_records as Scalar
    };

    QueryExecutionReport {
        results: batch_report.results,
        stats,
    }
}

/// Executes a pre-classified retained-leaf batch for benchmark diagnostics.
///
/// # Runtime Role
///
/// This helper isolates the retained-leaf execution portion of the query
/// pipeline after traversal has already selected candidate leaves. It is used by
/// benchmark debug output to estimate Stage II and Stage III cost without
/// changing the public query API or normal execution path.
///
/// # Formal Reference
///
/// This corresponds to the post-selection work in the staged FSE execution
/// model: deferred reconstruction followed by exact predicate evaluation.
pub(crate) fn execute_retained_leaf_batch_for_diagnostics(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    execute_classified_retained_leaves_with_candidate_count(
        index,
        query,
        retained_leaves,
        candidate_count,
        options,
    )
}

fn execute_query_into_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    results: &mut Vec<Vector>,
) -> QueryExecutionStats {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let root_classification = query.classify_bounds(&index.root_node().bounds);

    match root_classification {
        QueryBoundsClassification::Covered => {
            let result_buffer = std::mem::take(results);
            let batch_report =
                execute_fully_covered_index_serial_with_results(index, result_buffer);

            let stats = QueryExecutionStats {
                visited_nodes: 1,
                total_leaves: index.leaf_count(),
                retained_leaves: index.leaf_count(),
                retained_leaf_ratio: if index.leaf_count() == 0 { 0.0 } else { 1.0 },
                total_records: index.root_node().cardinality,
                reconstructed_records: batch_report.reconstructed_records,
                matched_records: batch_report.matched_records,
                candidate_ratio: if index.root_node().cardinality == 0 {
                    0.0
                } else {
                    batch_report.reconstructed_records as Scalar
                        / index.root_node().cardinality as Scalar
                },
            };

            *results = batch_report.results;

            return stats;
        }
        QueryBoundsClassification::Disjoint => {
            results.clear();

            return QueryExecutionStats {
                visited_nodes: 1,
                total_leaves: index.leaf_count(),
                retained_leaves: 0,
                retained_leaf_ratio: 0.0,
                total_records: index.root_node().cardinality,
                reconstructed_records: 0,
                matched_records: 0,
                candidate_ratio: 0.0,
            };
        }
        QueryBoundsClassification::Partial => {
            // normal path uses the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let mut stats = QueryExecutionStats {
        visited_nodes: traversal_report.stats.visited_nodes,
        total_leaves: traversal_report.stats.total_leaves,
        retained_leaves: traversal_report.stats.retained_leaves,
        retained_leaf_ratio: traversal_report.stats.retained_leaf_ratio,
        total_records: index.root_node().cardinality,
        ..QueryExecutionStats::default()
    };

    let result_buffer = std::mem::take(results);

    let batch_report = execute_classified_retained_leaves_serial_with_candidate_count_and_results(
        index,
        query,
        &traversal_report.retained_leaves,
        traversal_report.stats.retained_candidate_records,
        result_buffer,
    );

    stats.reconstructed_records = batch_report.reconstructed_records;
    stats.matched_records = batch_report.matched_records;

    stats.candidate_ratio = if stats.total_records == 0 {
        0.0
    } else {
        stats.reconstructed_records as Scalar / stats.total_records as Scalar
    };

    *results = batch_report.results;

    stats
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
        stats: QueryExecutionStats {
            visited_nodes: 1,
            total_leaves: index.leaf_count(),
            retained_leaves: 0,
            retained_leaf_ratio: 0.0,
            total_records: index.root_node().cardinality,
            reconstructed_records: 0,
            matched_records: 0,
            candidate_ratio: 0.0,
        },
    }
}
