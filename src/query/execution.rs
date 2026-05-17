//! End-to-end query execution.

use crate::math::{Scalar, Vector};
use crate::query::{QueryRegion, reconstruct_row_into, traverse_with_stats};
use crate::storage::{FSEIndex, PartitionNode};

/// Maximum number of result slots preallocated before exact evaluation.
///
/// # Runtime Role
///
/// Query execution can know the retained candidate count before exact filtering,
/// but that count is only an upper bound on final matches. This cap keeps
/// selective queries from allocating a large result buffer just because a
/// conservative bounding region retained many candidates.
pub(crate) const MAX_RESULT_PREALLOCATION: usize = 4096;

/// Execution strategy used by the query runtime.
///
/// # Runtime Role
///
/// `QueryExecutionMode` makes the retained-partition execution strategy explicit.
/// The current implementation supports only deterministic serial execution.
/// Parallel execution can be added later without changing the public query
/// result semantics.
///
/// # Formal Reference
///
/// This controls how retained partitions are processed after geometric
/// selection. It does not change the required semantic order:
///
/// `Geometry -> Reconstruction -> Logic`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryExecutionMode {
    /// Retained partitions are reconstructed and evaluated one at a time.
    Serial,
}

impl Default for QueryExecutionMode {
    fn default() -> Self {
        Self::Serial
    }
}

/// Options controlling query execution behavior.
///
/// # Runtime Role
///
/// `QueryExecutionOptions` provides a stable place to configure execution
/// strategy without changing the query API every time the runtime gains a new
/// execution mode.
///
/// The default is deterministic serial execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryExecutionOptions {
    /// Retained-partition execution strategy.
    pub mode: QueryExecutionMode,
}

impl QueryExecutionOptions {
    /// Creates options for deterministic serial query execution.
    pub fn serial() -> Self {
        Self {
            mode: QueryExecutionMode::Serial,
        }
    }
}

impl Default for QueryExecutionOptions {
    fn default() -> Self {
        Self::serial()
    }
}

/// Runtime statistics collected during query execution.
///
/// # Runtime Role
///
/// `QueryExecutionStats` provides visibility into how much work the FSE query
/// pipeline performs for a given query.
///
/// # Formal Reference
///
/// These values correspond to the runtime terms in the staged execution model:
/// metadata traversal, retained candidate partitions, deferred reconstruction,
/// and exact point-level evaluation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueryExecutionStats {
    /// Number of hierarchy nodes whose metadata was visited.
    pub visited_nodes: usize,

    /// Number of leaf partitions in the index.
    pub total_leaves: usize,

    /// Number of leaf partitions retained after metadata pruning.
    pub retained_leaves: usize,

    /// Fraction of leaf partitions retained after metadata pruning.
    pub retained_leaf_ratio: Scalar,

    /// Number of records represented by the index.
    pub total_records: usize,

    /// Number of records logically reconstructed after pruning.
    pub reconstructed_records: usize,

    /// Number of records returned after exact predicate evaluation.
    pub matched_records: usize,

    /// Fraction of total records reconstructed after pruning.
    pub candidate_ratio: Scalar,
}

/// Query result paired with execution statistics.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryExecutionReport {
    /// Exact query matches.
    pub results: Vec<Vector>,

    /// Runtime statistics for the query.
    pub stats: QueryExecutionStats,
}

/// Result of executing one retained leaf partition.
///
/// # Runtime Role
///
/// This report isolates Stage II and Stage III work for a single retained leaf.
/// The structure is intentionally local to query execution so retained leaves
/// can later be evaluated independently without changing query semantics.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RetainedLeafExecutionReport {
    /// Exact matches produced by this retained leaf.
    pub(crate) results: Vec<Vector>,

    /// Number of records reconstructed from this retained leaf.
    pub(crate) reconstructed_records: usize,

    /// Number of records that matched the exact query predicate.
    pub(crate) matched_records: usize,
}

/// Result of executing all retained leaf partitions.
///
/// # Runtime Role
///
/// This report aggregates the Stage II and Stage III work performed across a
/// retained leaf batch. It is intentionally separate from traversal statistics
/// so the retained-partition execution strategy can evolve independently.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RetainedLeafBatchExecutionReport {
    /// Exact matches produced by all retained leaves in the batch.
    pub(crate) results: Vec<Vector>,

    /// Number of records reconstructed across retained leaves.
    pub(crate) reconstructed_records: usize,

    /// Number of records that matched the exact query predicate.
    pub(crate) matched_records: usize,
}

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
/// exact query semantics. At this stage, only serial execution is supported.
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
/// Candidate rows are reconstructed into a reusable coordinate buffer and then
/// evaluated immediately. An owned `Vector` is allocated only when the row
/// satisfies the exact query predicate.
///
/// Execution options control how retained leaves are processed after traversal.
/// The only currently supported mode is serial execution.
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
    let traversal_report = traverse_with_stats(index, query);

    let mut stats = QueryExecutionStats {
        visited_nodes: traversal_report.stats.visited_nodes,
        total_leaves: traversal_report.stats.total_leaves,
        retained_leaves: traversal_report.stats.retained_leaves,
        retained_leaf_ratio: traversal_report.stats.retained_leaf_ratio,
        total_records: index.root_node().cardinality,
        ..QueryExecutionStats::default()
    };

    let batch_report = execute_retained_leaves_with_options(
        index,
        query,
        &traversal_report.retained_leaf_ids,
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

/// Executes Stage II and Stage III for all retained leaves using default options.
///
/// # Runtime Role
///
/// This helper preserves the existing serial retained-leaf batch API while the
/// execution-options seam is introduced.
pub(crate) fn execute_retained_leaves(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    execute_retained_leaves_with_options(
        index,
        query,
        retained_leaf_ids,
        QueryExecutionOptions::default(),
    )
}

/// Executes Stage II and Stage III for all retained leaves using explicit options.
///
/// # Runtime Role
///
/// This helper dispatches retained-partition execution based on the selected
/// execution mode. It is intentionally small so additional modes can be added
/// without changing traversal or final query reporting.
///
/// # Formal Reference
///
/// This performs the retained-partition portion of:
///
/// `Reconstruction -> Logic -> Merge`.
///
/// The function assumes geometric pruning has already produced the retained
/// leaf identifiers.
///
/// # Panics
///
/// Panics if any retained node identifier is out of range or points to a
/// non-leaf node.
pub(crate) fn execute_retained_leaves_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    match options.mode {
        QueryExecutionMode::Serial => {
            execute_retained_leaves_serial(index, query, retained_leaf_ids)
        }
    }
}

/// Executes retained leaves using deterministic serial iteration.
///
/// # Runtime Role
///
/// This is the current production execution path for retained leaves. It is
/// separate from the options dispatcher so later parallel execution can be added
/// beside it instead of being mixed into the serial loop.
///
/// # Panics
///
/// Panics if any retained node identifier is out of range or points to a
/// non-leaf node.
pub(crate) fn execute_retained_leaves_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    validate_retained_leaf_ids(index, retained_leaf_ids);

    let candidate_count = retained_candidate_count(index, retained_leaf_ids);
    let mut leaf_reports = Vec::with_capacity(retained_leaf_ids.len());

    // still serial on purpose
    for node_id in retained_leaf_ids {
        let node = &index.nodes[*node_id];
        leaf_reports.push(execute_retained_leaf(node, query, index.dimensions));
    }

    let batch_report = merge_retained_leaf_reports_in_order(leaf_reports, candidate_count);

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "retained candidate count should match reconstructed retained rows"
    );

    batch_report
}

/// Validates that retained node identifiers reference leaf partitions.
///
/// # Runtime Role
///
/// Retained leaf execution assumes Stage I traversal already selected valid
/// leaf partitions. This helper makes that boundary explicit so later execution
/// strategies can focus on reconstruction and evaluation instead of defensive
/// index checks.
///
/// # Panics
///
/// Panics when a retained node identifier is outside the index or references an
/// internal partition.
pub(crate) fn validate_retained_leaf_ids(index: &FSEIndex, retained_leaf_ids: &[usize]) {
    // dont let later paralel work inherit sketchy ids
    for node_id in retained_leaf_ids {
        let Some(node) = index.nodes.get(*node_id) else {
            panic!("retained leaf id {node_id} is outside index node range");
        };

        assert!(
            node.is_leaf,
            "retained leaf id {node_id} must reference a leaf partition"
        );
    }
}

/// Executes Stage II and Stage III for one retained leaf partition.
///
/// # Runtime Role
///
/// This helper reconstructs candidate records from one retained leaf and applies
/// the exact query predicate immediately after each row is lifted back into
/// coordinate space.
///
/// # Formal Reference
///
/// This implements the per-leaf portion of:
///
/// `Reconstruction -> Logic`.
///
/// The function assumes geometric pruning has already retained the leaf.
///
/// # Panics
///
/// Panics when `node` is not a leaf partition.
pub(crate) fn execute_retained_leaf(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
) -> RetainedLeafExecutionReport {
    assert!(
        node.is_leaf,
        "retained leaf execution helper requires a leaf node"
    );

    let row_count = node.residuals.cardinality();
    let mut results = Vec::with_capacity(result_capacity_hint(row_count));
    let mut reconstructed_values = Vec::with_capacity(dimensions);

    // local results get a sane head start now too
    for row in 0..row_count {
        reconstruct_row_into(node, row, &mut reconstructed_values);

        if query.contains_values(&reconstructed_values) {
            results.push(Vector::new(reconstructed_values.clone()));
        }
    }

    RetainedLeafExecutionReport {
        reconstructed_records: row_count,
        matched_records: results.len(),
        results,
    }
}

/// Merges retained leaf reports in their supplied order.
///
/// # Runtime Role
///
/// This helper defines the deterministic merge contract for retained-leaf
/// execution. Serial execution supplies reports in retained traversal order.
/// Future parallel execution should compute reports independently but still pass
/// them to this function in retained leaf order before final result assembly.
///
/// # Formal Reference
///
/// This is the merge portion of:
///
/// `Reconstruction -> Logic -> Merge`.
///
/// It does not change reconstruction or exact predicate semantics.
pub(crate) fn merge_retained_leaf_reports_in_order(
    leaf_reports: Vec<RetainedLeafExecutionReport>,
    candidate_count: usize,
) -> RetainedLeafBatchExecutionReport {
    let mut results = Vec::with_capacity(result_capacity_hint(candidate_count));
    let mut aggregate_stats = QueryExecutionStats::default();

    // keep report order boring and obvious
    for leaf_report in leaf_reports {
        merge_retained_leaf_report(&mut results, &mut aggregate_stats, leaf_report);
    }

    RetainedLeafBatchExecutionReport {
        results,
        reconstructed_records: aggregate_stats.reconstructed_records,
        matched_records: aggregate_stats.matched_records,
    }
}

/// Merges one retained leaf report into the final query result.
///
/// # Runtime Role
///
/// This helper keeps result merging and execution-stat aggregation in one place.
/// It is intentionally serial for now, but it provides a clear seam for later
/// retained-partition parallel execution.
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

/// Reserves enough final result capacity for an incoming leaf result batch.
///
/// # Runtime Role
///
/// The final query result vector may start with a bounded capacity hint. If
/// actual matches exceed that initial hint, this helper reserves exactly the
/// additional space needed before merging the next retained leaf report.
pub(crate) fn reserve_additional_results(results: &mut Vec<Vector>, incoming_len: usize) {
    let available_capacity = results.capacity().saturating_sub(results.len());

    if incoming_len > available_capacity {
        // just enough room for this batch
        results.reserve_exact(incoming_len - available_capacity);
    }
}

/// Returns the number of records contained in retained leaves.
///
/// # Runtime Role
///
/// This count is the maximum number of rows that can be returned by exact
/// evaluation after geometric pruning. It is also the number of rows that Stage
/// II will reconstruct.
///
/// # Panics
///
/// Panics if any retained node identifier is outside the index node range.
pub(crate) fn retained_candidate_count(index: &FSEIndex, retained_leaf_ids: &[usize]) -> usize {
    retained_leaf_ids
        .iter()
        .map(|node_id| index.nodes[*node_id].residuals.cardinality())
        .sum()
}

/// Returns a bounded capacity hint for final query results.
///
/// # Runtime Role
///
/// The retained candidate count is an upper bound, not a prediction of final
/// matches. This helper avoids repeated result-vector growth for common result
/// sets while avoiding huge allocations for conservative retained partitions.
pub(crate) fn result_capacity_hint(retained_candidate_count: usize) -> usize {
    retained_candidate_count.min(MAX_RESULT_PREALLOCATION)
}
