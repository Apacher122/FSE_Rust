//! Retained-leaf result merging.

use crate::math::Vector;

use super::super::reports::{
    QueryExecutionStats, RetainedLeafBatchExecutionReport, RetainedLeafExecutionReport,
    result_capacity_hint,
};

/// Merges retained leaf reports in their supplied order.
///
/// # Runtime Role
///
/// This helper defines the deterministic merge contract for retained-leaf
/// execution. Parallel execution computes reports independently but still passes
/// them to this function in retained leaf order before final result assembly.
pub(crate) fn merge_retained_leaf_reports_in_order(
    leaf_reports: Vec<RetainedLeafExecutionReport>,
    candidate_count: usize,
) -> RetainedLeafBatchExecutionReport {
    let mut results = Vec::with_capacity(result_capacity_hint(candidate_count));
    let mut aggregate_stats = QueryExecutionStats::default();

    #[cfg(test)]
    let mut predicate_evaluated_records = 0;

    // parallel reports still merge here
    for leaf_report in leaf_reports {
        #[cfg(test)]
        {
            predicate_evaluated_records += leaf_report.predicate_evaluated_records;
        }

        merge_retained_leaf_report(&mut results, &mut aggregate_stats, leaf_report);
    }

    RetainedLeafBatchExecutionReport {
        results,
        reconstructed_records: aggregate_stats.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records,
        matched_records: aggregate_stats.matched_records,
    }
}

/// Merges one retained leaf report into the final query result.
///
/// # Runtime Role
///
/// This helper keeps result merging and execution-stat aggregation in one place
/// for the parallel path, where leaf-local result vectors are still required.
pub(crate) fn merge_retained_leaf_report(
    results: &mut Vec<Vector>,
    stats: &mut QueryExecutionStats,
    leaf_report: RetainedLeafExecutionReport,
) {
    let incoming_results = leaf_report.results.len();

    stats.reconstructed_records += leaf_report.reconstructed_records;
    stats.matched_records += leaf_report.matched_records;

    // merge step gets its own small seam now
    reserve_additional_results(results, incoming_results);
    results.extend(leaf_report.results);
}

/// Reserves enough final result capacity for an incoming result batch.
///
/// # Runtime Role
///
/// The final query result vector may start with a bounded capacity hint. If
/// actual matches exceed that initial hint, this helper reserves exactly the
/// additional space needed before appending more results.
pub(crate) fn reserve_additional_results(results: &mut Vec<Vector>, incoming_len: usize) {
    let available_capacity = results.capacity().saturating_sub(results.len());

    if incoming_len > available_capacity {
        // just enough room for this batch
        results.reserve_exact(incoming_len - available_capacity);
    }
}
