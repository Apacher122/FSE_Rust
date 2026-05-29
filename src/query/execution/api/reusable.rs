//! Reusable owned-result query execution API.

use crate::math::Vector;
use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::query::traversal::traverse_with_known_root_classification;
use crate::storage::FSEIndex;

use super::super::options::{QueryExecutionMode, QueryExecutionOptions};
use super::super::reports::QueryExecutionStats;
use super::super::root::classify_query_root;
use super::super::root_coverage::execute_fully_covered_index_serial_with_results;
use super::super::serial::execute_classified_retained_leaves_serial_with_candidate_count_and_results;
use super::super::stats::{
    apply_batch_report_to_stats, root_covered_stats, root_disjoint_stats, stats_from_traversal,
};
use super::owned::execute_query_with_stats_and_options;

/// Executes a query into a caller-provided result buffer.
///
/// # Runtime Role
///
/// This is the reusable-buffer owned-result API. It preserves the same exact
/// query semantics as [`super::owned::execute_query`], but writes the results
/// into `results` instead of returning a freshly allocated outer `Vec<Vector>`.
///
/// Existing capacity may be reused.
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

fn execute_query_into_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    results: &mut Vec<Vector>,
) -> QueryExecutionStats {
    let root_classification = classify_query_root(index, query);

    match root_classification {
        QueryBoundsClassification::Covered => {
            let result_buffer = std::mem::take(results);
            let batch_report =
                execute_fully_covered_index_serial_with_results(index, result_buffer);
            let stats = root_covered_stats(index, &batch_report);

            *results = batch_report.results;

            return stats;
        }
        QueryBoundsClassification::Disjoint => {
            results.clear();

            return root_disjoint_stats(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path uses the root classification we already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let mut stats = stats_from_traversal(index, &traversal_report);
    let result_buffer = std::mem::take(results);

    let batch_report = execute_classified_retained_leaves_serial_with_candidate_count_and_results(
        index,
        query,
        &traversal_report.retained_leaves,
        traversal_report.stats.retained_candidate_records,
        result_buffer,
    );

    apply_batch_report_to_stats(&mut stats, &batch_report);

    *results = batch_report.results;

    stats
}
