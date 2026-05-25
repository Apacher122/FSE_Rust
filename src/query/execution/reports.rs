//! Query execution report types.

use crate::math::{Scalar, Vector};

/// Maximum number of result slots preallocated before exact evaluation.
///
/// # Runtime Role
///
/// Query execution can know the retained candidate count before exact filtering,
/// but that count is only an upper bound on final matches. This cap keeps
/// selective queries from allocating a large result buffer just because a
/// conservative bounding region retained many candidates.
pub(crate) const MAX_RESULT_PREALLOCATION: usize = 4096;

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

/// Count-only query result paired with execution statistics.
///
/// # Runtime Role
///
/// `QueryCountReport` exposes exact query cardinality without materializing
/// owned result vectors. This keeps the same structural accounting as
/// `QueryExecutionReport` while avoiding result ownership costs.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryCountReport {
    /// Number of exact records matched by the query.
    pub matched_records: usize,

    /// Runtime statistics for the count-only query.
    pub stats: QueryExecutionStats,
}

/// Exact reference to a matching record inside the FSE index.
///
/// # Runtime Role
///
/// `QueryResultReference` identifies a matching row without materializing an
/// owned [`Vector`]. This gives callers a lower-allocation output contract when
/// they need exact match identity but do not immediately need reconstructed row
/// values.
///
/// # Formal Reference
///
/// The referenced row has already passed the same staged execution semantics as
/// owned-result queries:
///
/// `Geometry -> Reconstruction -> Logic`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryResultReference {
    /// Leaf node containing the matching residual row.
    pub node_id: usize,

    /// Row index inside the leaf residual block.
    pub row_index: usize,
}

/// Query reference result paired with execution statistics.
///
/// # Runtime Role
///
/// `QueryReferenceReport` exposes exact matching row references while preserving
/// the same structural accounting used by owned-result and count-only query
/// execution.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryReferenceReport {
    /// Exact matching row references.
    pub matches: Vec<QueryResultReference>,

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

    /// Creates an empty batch report from a caller-owned result buffer.
    ///
    /// # Runtime Role
    ///
    /// This constructor lets owned-result query execution reuse the outer
    /// `Vec<Vector>` allocation across repeated exact queries. The buffer is
    /// cleared before use and then reserved up to the normal bounded capacity hint.
    ///
    /// This does not remove per-row `Vector` materialization. It only avoids
    /// repeatedly allocating the outer result collection.
    pub(crate) fn with_result_buffer(candidate_count: usize, mut results: Vec<Vector>) -> Self {
        results.clear();

        let target_capacity = result_capacity_hint(candidate_count);

        if results.capacity() < target_capacity {
            results.reserve_exact(target_capacity - results.capacity());
        }

        Self {
            results,
            reconstructed_records: 0,
            #[cfg(test)]
            predicate_evaluated_records: 0,
            matched_records: 0,
        }
    }
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
