//! End-to-end query execution.

use rayon::prelude::*;

#[cfg(test)]
use crate::math::BoundingBox;
use crate::math::{Scalar, Vector};
use crate::query::{
    QueryRegion, RetainedLeaf, RetainedLeafCoverage, reconstruct_row_into, traverse_with_stats,
};
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

/// Default retained-leaf threshold required before parallel execution uses Rayon.
///
/// # Runtime Role
///
/// Parallel retained-leaf execution has scheduling overhead. This threshold
/// keeps small retained-leaf batches on the deterministic serial path while
/// still allowing larger batches to use Rayon.
pub const DEFAULT_PARALLEL_MIN_RETAINED_LEAVES: usize = 4;

/// Execution strategy used by the query runtime.
///
/// # Runtime Role
///
/// `QueryExecutionMode` makes the retained-partition execution strategy explicit.
/// Serial execution processes retained partitions one at a time. Parallel
/// execution evaluates retained leaf partitions independently before merging
/// their local reports in deterministic retained-leaf order.
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

    /// Retained partitions are reconstructed and evaluated independently using Rayon.
    Parallel,
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

    /// Minimum retained-leaf count required before parallel mode uses Rayon.
    ///
    /// This value is ignored by serial mode.
    pub parallel_min_retained_leaves: usize,
}

impl QueryExecutionOptions {
    /// Creates options for deterministic serial query execution.
    pub fn serial() -> Self {
        Self {
            mode: QueryExecutionMode::Serial,
            parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
    }

    /// Creates options for parallel retained-partition query execution.
    ///
    /// # Runtime Role
    ///
    /// Parallel execution evaluates retained leaf partitions independently while
    /// preserving deterministic final result ordering through ordered report
    /// collection and merge. Small retained-leaf batches fall back to serial
    /// execution based on `parallel_min_retained_leaves`.
    pub fn parallel() -> Self {
        Self {
            mode: QueryExecutionMode::Parallel,
            parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
    }

    /// Returns a copy of the options with a new parallel retained-leaf threshold.
    ///
    /// # Runtime Role
    ///
    /// This allows benchmarks and tests to tune the point where parallel mode
    /// starts using Rayon without changing the selected execution mode.
    pub fn with_parallel_min_retained_leaves(mut self, threshold: usize) -> Self {
        self.parallel_min_retained_leaves = threshold;
        self
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
/// can be evaluated independently without changing query semantics.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RetainedLeafExecutionReport {
    /// Exact matches produced by this retained leaf.
    pub(crate) results: Vec<Vector>,

    /// Number of records reconstructed from this retained leaf.
    pub(crate) reconstructed_records: usize,

    /// Number of records that required exact predicate checks.
    ///
    /// # Runtime Role
    ///
    /// This field is test-only because current benchmark accounting does not
    /// expose predicate-check counts yet.
    #[cfg(test)]
    pub(crate) predicate_evaluated_records: usize,

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

    /// Number of records that required exact predicate checks.
    ///
    /// # Runtime Role
    ///
    /// This field is test-only until predicate-check counts become part of
    /// benchmark-facing execution stats.
    #[cfg(test)]
    pub(crate) predicate_evaluated_records: usize,

    /// Number of records that matched the exact query predicate.
    pub(crate) matched_records: usize,
}

impl RetainedLeafBatchExecutionReport {
    /// Creates an empty batch report with bounded result capacity.
    ///
    /// # Runtime Role
    ///
    /// Serial retained-leaf execution streams directly into this report instead
    /// of allocating one local result vector per leaf.
    pub(crate) fn with_candidate_capacity(candidate_count: usize) -> Self {
        Self {
            results: Vec::with_capacity(result_capacity_hint(candidate_count)),
            reconstructed_records: 0,
            #[cfg(test)]
            predicate_evaluated_records: 0,
            matched_records: 0,
        }
    }
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

    if query.contains_bounds(&index.root_node().bounds) {
        // root coverage means the whole index is already proven in range
        return execute_fully_covered_index_with_options(index, query, options);
    }

    let traversal_report = traverse_with_stats(index, query);

    let mut stats = QueryExecutionStats {
        visited_nodes: traversal_report.stats.visited_nodes,
        total_leaves: traversal_report.stats.total_leaves,
        retained_leaves: traversal_report.stats.retained_leaves,
        retained_leaf_ratio: traversal_report.stats.retained_leaf_ratio,
        total_records: index.root_node().cardinality,
        ..QueryExecutionStats::default()
    };

    let batch_report = execute_classified_retained_leaves_with_options(
        index,
        query,
        &traversal_report.retained_leaves,
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

/// Executes a query that fully contains the root bounding region.
///
/// # Runtime Role
///
/// This is the full-index coverage fast path. It bypasses normal traversal
/// because the root bound already proves every indexed record satisfies the
/// query.
///
/// Serial mode streams all leaf rows directly into one output buffer. Parallel
/// mode still uses the classified retained-leaf execution path so the requested
/// execution strategy is preserved for larger datasets.
pub(crate) fn execute_fully_covered_index_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    options: QueryExecutionOptions,
) -> QueryExecutionReport {
    let total_leaves = leaf_count(index);
    let total_records = index.root_node().cardinality;

    let batch_report = match options.mode {
        QueryExecutionMode::Serial => execute_fully_covered_index_serial(index),
        QueryExecutionMode::Parallel => {
            let retained_leaves = fully_covered_retained_leaves(index);
            execute_classified_retained_leaves_with_options(index, query, &retained_leaves, options)
        }
    };

    let stats = QueryExecutionStats {
        visited_nodes: 1,
        total_leaves,
        retained_leaves: total_leaves,
        retained_leaf_ratio: if total_leaves == 0 { 0.0 } else { 1.0 },
        total_records,
        reconstructed_records: batch_report.reconstructed_records,
        matched_records: batch_report.matched_records,
        candidate_ratio: if total_records == 0 {
            0.0
        } else {
            batch_report.reconstructed_records as Scalar / total_records as Scalar
        },
    };

    QueryExecutionReport {
        results: batch_report.results,
        stats,
    }
}

/// Executes a fully covered index using direct serial leaf streaming.
///
/// # Runtime Role
///
/// This function skips traversal-produced retained-leaf vectors entirely and
/// reconstructs every leaf row into one batch result.
///
/// # Panics
///
/// Panics when the sum of reconstructed leaf rows does not match root
/// cardinality in debug builds.
pub(crate) fn execute_fully_covered_index_serial(
    index: &FSEIndex,
) -> RetainedLeafBatchExecutionReport {
    let candidate_count = index.root_node().cardinality;
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(candidate_count);
    let mut reconstructed_values = Vec::with_capacity(index.dimensions);

    // one buffer for the whole full index path
    for node in index.nodes.iter().filter(|node| node.is_leaf) {
        append_covered_retained_leaf_results(node, &mut batch_report, &mut reconstructed_values);
    }

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "fully covered index reconstruction should match root cardinality"
    );

    batch_report
}

/// Returns retained-leaf records for every leaf in the index.
///
/// # Runtime Role
///
/// Parallel fully covered queries still need retained-leaf work units. Every
/// leaf is classified as covered because root containment proves all descendants
/// are covered.
pub(crate) fn fully_covered_retained_leaves(index: &FSEIndex) -> Vec<RetainedLeaf> {
    index
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(node_id, node)| {
            if node.is_leaf {
                Some(RetainedLeaf::covered(node_id))
            } else {
                None
            }
        })
        .collect()
}

/// Returns the number of leaf partitions in an index.
pub(crate) fn leaf_count(index: &FSEIndex) -> usize {
    index.nodes.iter().filter(|node| node.is_leaf).count()
}

/// Executes Stage II and Stage III for all retained leaves using default options.
///
/// # Runtime Role
///
/// This helper preserves the default retained-leaf batch API while classifying
/// retained leaf identifiers before execution.
///
/// # Notes
///
/// This exists for internal tests that still exercise the id-based retained-leaf
/// API. Normal query execution consumes traversal-classified retained leaves.
#[cfg(test)]
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

/// Executes Stage II and Stage III for retained leaf identifiers using explicit options.
///
/// # Runtime Role
///
/// This compatibility helper accepts retained leaf identifiers and performs the
/// coverage classification needed by the newer retained-leaf execution path.
#[cfg(test)]
pub(crate) fn execute_retained_leaves_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    execute_classified_retained_leaves_with_options(index, query, &retained_leaves, options)
}

/// Executes already classified retained leaves using explicit options.
///
/// # Runtime Role
///
/// Query execution uses this path directly because traversal already knows
/// whether each retained leaf is covered or partial.
pub(crate) fn execute_classified_retained_leaves_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    match options.mode {
        QueryExecutionMode::Serial => {
            execute_classified_retained_leaves_serial(index, query, retained_leaves)
        }
        QueryExecutionMode::Parallel => {
            if should_execute_retained_leaves_in_parallel(options, retained_leaves.len()) {
                execute_classified_retained_leaves_parallel(index, query, retained_leaves)
            } else {
                // rayon is not free
                execute_classified_retained_leaves_serial(index, query, retained_leaves)
            }
        }
    }
}

/// Returns true when parallel mode should use Rayon for the retained-leaf batch.
///
/// # Runtime Role
///
/// This policy prevents small retained-leaf batches from paying parallel
/// scheduling overhead. Serial mode always returns false.
pub(crate) fn should_execute_retained_leaves_in_parallel(
    options: QueryExecutionOptions,
    retained_leaf_count: usize,
) -> bool {
    matches!(options.mode, QueryExecutionMode::Parallel)
        && retained_leaf_count >= options.parallel_min_retained_leaves
}

/// Executes retained leaf identifiers using deterministic serial iteration.
///
/// # Runtime Role
///
/// This preserves the older test/helper API by classifying leaf ids before
/// dispatching to the classified retained-leaf execution path.
#[cfg(test)]
pub(crate) fn execute_retained_leaves_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    execute_classified_retained_leaves_serial(index, query, &retained_leaves)
}

/// Executes classified retained leaves using deterministic serial iteration.
///
/// # Runtime Role
///
/// This is the deterministic single-threaded retained-leaf execution path. It
/// streams all retained rows into one batch report and avoids per-leaf result
/// vectors on the serial path.
pub(crate) fn execute_classified_retained_leaves_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
) -> RetainedLeafBatchExecutionReport {
    validate_retained_leaves(index, retained_leaves);

    let candidate_count = classified_retained_candidate_count(index, retained_leaves);
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(candidate_count);
    let mut reconstructed_values = Vec::with_capacity(index.dimensions);

    // one scratch buffer for every retained leaf in this serial query
    for retained_leaf in retained_leaves {
        let node = &index.nodes[retained_leaf.node_id];
        execute_retained_leaf_into_batch_report(
            node,
            query,
            retained_leaf.coverage,
            &mut batch_report,
            &mut reconstructed_values,
        );
    }

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "retained candidate count should match reconstructed retained rows"
    );

    batch_report
}

/// Executes retained leaf identifiers using Rayon-backed parallel iteration.
///
/// # Runtime Role
///
/// This preserves the older test/helper API by classifying leaf ids before
/// dispatching to the classified retained-leaf execution path.
#[cfg(test)]
pub(crate) fn execute_retained_leaves_parallel(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    execute_classified_retained_leaves_parallel(index, query, &retained_leaves)
}

/// Executes classified retained leaves using Rayon-backed parallel iteration.
///
/// # Runtime Role
///
/// This is the parallel retained-partition execution path. Each retained leaf
/// reconstructs and evaluates its own rows independently. The resulting reports
/// are collected in retained-leaf order and then merged deterministically.
pub(crate) fn execute_classified_retained_leaves_parallel(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
) -> RetainedLeafBatchExecutionReport {
    validate_retained_leaves(index, retained_leaves);

    let candidate_count = classified_retained_candidate_count(index, retained_leaves);

    // rayon collect preserves order for this indexed slice iterator
    let leaf_reports: Vec<RetainedLeafExecutionReport> = retained_leaves
        .par_iter()
        .map(|retained_leaf| {
            let node = &index.nodes[retained_leaf.node_id];

            // parallel still needs leaf local buffers
            execute_retained_leaf_with_coverage(
                node,
                query,
                index.dimensions,
                retained_leaf.coverage,
            )
        })
        .collect();

    let batch_report = merge_retained_leaf_reports_in_order(leaf_reports, candidate_count);

    debug_assert_eq!(
        batch_report.reconstructed_records, candidate_count,
        "retained candidate count should match reconstructed retained rows"
    );

    batch_report
}

/// Converts retained leaf identifiers into traversal-style retained leaf records.
///
/// # Runtime Role
///
/// This helper keeps older internal tests and helpers working while allowing
/// normal query execution to consume classified traversal output directly.
#[cfg(test)]
pub(crate) fn classify_retained_leaf_ids(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> Vec<RetainedLeaf> {
    validate_retained_leaf_ids(index, retained_leaf_ids);

    retained_leaf_ids
        .iter()
        .map(|node_id| {
            let node = &index.nodes[*node_id];

            if query.contains_bounds(&node.bounds) {
                RetainedLeaf::covered(*node_id)
            } else {
                RetainedLeaf::partial(*node_id)
            }
        })
        .collect()
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
#[cfg(test)]
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

/// Validates that retained leaf records reference leaf partitions.
///
/// # Runtime Role
///
/// Classified traversal output should already be valid, but this keeps execution
/// helpers safe when tests or future callers construct retained leaves directly.
pub(crate) fn validate_retained_leaves(index: &FSEIndex, retained_leaves: &[RetainedLeaf]) {
    for retained_leaf in retained_leaves {
        let Some(node) = index.nodes.get(retained_leaf.node_id) else {
            panic!(
                "retained leaf id {} is outside index node range",
                retained_leaf.node_id
            );
        };

        assert!(
            node.is_leaf,
            "retained leaf id {} must reference a leaf partition",
            retained_leaf.node_id
        );
    }
}

/// Executes Stage II and Stage III for one retained leaf partition.
///
/// # Runtime Role
///
/// This compatibility helper classifies the retained leaf locally. Normal query
/// execution should prefer traversal-provided classification.
///
/// # Panics
///
/// Panics when `node` is not a leaf partition.
#[cfg(test)]
pub(crate) fn execute_retained_leaf(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
) -> RetainedLeafExecutionReport {
    let coverage = if query.contains_bounds(&node.bounds) {
        RetainedLeafCoverage::Covered
    } else {
        RetainedLeafCoverage::Partial
    };

    execute_retained_leaf_with_coverage(node, query, dimensions, coverage)
}

/// Executes Stage II and Stage III for one retained leaf with known coverage.
///
/// # Runtime Role
///
/// The coverage classification comes from traversal in the normal query path.
/// Covered leaves skip exact per-row predicate checks. Partial leaves preserve
/// the exact predicate path.
pub(crate) fn execute_retained_leaf_with_coverage(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
    coverage: RetainedLeafCoverage,
) -> RetainedLeafExecutionReport {
    assert!(
        node.is_leaf,
        "retained leaf execution helper requires a leaf node"
    );

    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(node.residuals.cardinality());
    let mut reconstructed_values = Vec::with_capacity(dimensions);

    execute_retained_leaf_into_batch_report(
        node,
        query,
        coverage,
        &mut batch_report,
        &mut reconstructed_values,
    );

    RetainedLeafExecutionReport {
        results: batch_report.results,
        reconstructed_records: batch_report.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records: batch_report.predicate_evaluated_records,
        matched_records: batch_report.matched_records,
    }
}

/// Streams one retained leaf into an existing batch report.
///
/// # Runtime Role
///
/// This is the serial execution hot path. It preserves retained-leaf ordering
/// while avoiding a temporary result vector and merge step for each leaf.
pub(crate) fn execute_retained_leaf_into_batch_report(
    node: &PartitionNode,
    query: &QueryRegion,
    coverage: RetainedLeafCoverage,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    assert!(
        node.is_leaf,
        "retained leaf streaming helper requires a leaf node"
    );

    match coverage {
        RetainedLeafCoverage::Covered => {
            append_covered_retained_leaf_results(node, batch_report, reconstructed_values)
        }
        RetainedLeafCoverage::Partial => append_partially_covered_retained_leaf_results(
            node,
            query,
            batch_report,
            reconstructed_values,
        ),
    }
}

/// Appends all rows from a covered retained leaf into the batch report.
///
/// # Runtime Role
///
/// The query already contains the leaf bounds, so every reconstructed row can be
/// appended directly without exact predicate evaluation.
pub(crate) fn append_covered_retained_leaf_results(
    node: &PartitionNode,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    let row_count = node.residuals.cardinality();

    reserve_additional_results(&mut batch_report.results, row_count);

    // geometry already proved these rows match
    for row in 0..row_count {
        reconstruct_row_into(node, row, reconstructed_values);
        batch_report
            .results
            .push(Vector::new(reconstructed_values.clone()));
    }

    batch_report.reconstructed_records += row_count;
    batch_report.matched_records += row_count;
}

/// Appends matching rows from a partially covered retained leaf into the batch report.
///
/// # Runtime Role
///
/// Partial leaves still use the exact predicate path. The reconstructed row
/// buffer is reused across all rows in the leaf.
pub(crate) fn append_partially_covered_retained_leaf_results(
    node: &PartitionNode,
    query: &QueryRegion,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    let row_count = node.residuals.cardinality();
    let original_match_count = batch_report.results.len();

    // exact path stays here no shortcuts for partial leaves
    for row in 0..row_count {
        reconstruct_row_into(node, row, reconstructed_values);

        if query.contains_values(reconstructed_values) {
            batch_report
                .results
                .push(Vector::new(reconstructed_values.clone()));
        }
    }

    let matched_records = batch_report.results.len() - original_match_count;

    batch_report.reconstructed_records += row_count;
    #[cfg(test)]
    {
        batch_report.predicate_evaluated_records += row_count;
    }
    batch_report.matched_records += matched_records;
}

/// Executes a retained leaf whose bounding box is fully contained by the query.
///
/// # Runtime Role
///
/// Covered leaves skip exact per-row predicate checks. Reconstruction still
/// happens because query output is expressed as coordinate vectors, but every
/// reconstructed row can be appended directly to the result set.
#[cfg(test)]
pub(crate) fn execute_covered_retained_leaf(
    node: &PartitionNode,
    dimensions: usize,
) -> RetainedLeafExecutionReport {
    assert!(
        node.is_leaf,
        "covered retained leaf helper requires a leaf node"
    );

    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(node.residuals.cardinality());
    let mut reconstructed_values = Vec::with_capacity(dimensions);

    append_covered_retained_leaf_results(node, &mut batch_report, &mut reconstructed_values);

    RetainedLeafExecutionReport {
        results: batch_report.results,
        reconstructed_records: batch_report.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records: batch_report.predicate_evaluated_records,
        matched_records: batch_report.matched_records,
    }
}

/// Executes a retained leaf whose bounding box only partially overlaps the query.
///
/// # Runtime Role
///
/// Partially covered leaves preserve the full exact predicate path. Each row is
/// reconstructed into a reusable buffer and only materialized as an owned
/// `Vector` after passing the query predicate.
#[cfg(test)]
pub(crate) fn execute_partially_covered_retained_leaf(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
) -> RetainedLeafExecutionReport {
    assert!(
        node.is_leaf,
        "partial retained leaf helper requires a leaf node"
    );

    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(node.residuals.cardinality());
    let mut reconstructed_values = Vec::with_capacity(dimensions);

    append_partially_covered_retained_leaf_results(
        node,
        query,
        &mut batch_report,
        &mut reconstructed_values,
    );

    RetainedLeafExecutionReport {
        results: batch_report.results,
        reconstructed_records: batch_report.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records: batch_report.predicate_evaluated_records,
        matched_records: batch_report.matched_records,
    }
}

/// Returns whether a query region fully contains a bounding box.
///
/// # Runtime Role
///
/// This predicate preserves the previous execution helper API while delegating
/// the actual containment logic to `QueryRegion`.
///
/// # Panics
///
/// Panics when dimensionality differs between the query and bounds.
#[cfg(test)]
pub(crate) fn query_contains_bounds(query: &QueryRegion, bounds: &BoundingBox) -> bool {
    assert_eq!(
        query.dimensions(),
        bounds.dimensions(),
        "query and bounds dimensionality must match"
    );

    query.contains_bounds(bounds)
}

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
#[cfg(test)]
pub(crate) fn retained_candidate_count(index: &FSEIndex, retained_leaf_ids: &[usize]) -> usize {
    retained_leaf_ids
        .iter()
        .map(|node_id| index.nodes[*node_id].residuals.cardinality())
        .sum()
}

/// Returns the number of records contained in classified retained leaves.
///
/// # Runtime Role
///
/// This is the classified retained-leaf equivalent of `retained_candidate_count`.
pub(crate) fn classified_retained_candidate_count(
    index: &FSEIndex,
    retained_leaves: &[RetainedLeaf],
) -> usize {
    retained_leaves
        .iter()
        .map(|retained_leaf| index.nodes[retained_leaf.node_id].residuals.cardinality())
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
