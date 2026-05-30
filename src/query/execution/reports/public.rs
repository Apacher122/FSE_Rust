//! Public query execution report types.

use crate::math::{Scalar, Vector};

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

    /// Number of records reconstructed or inspected after pruning.
    ///
    /// Full-result, reference-result, and count-only execution report the full
    /// retained candidate count. Short-circuit output contracts report the
    /// number of candidate records needed to establish their result.
    pub reconstructed_records: usize,

    /// Number of records accepted by exact predicate evaluation for this report.
    ///
    /// Full-result, reference-result, and count-only execution report exact
    /// cardinality. Short-circuit output contracts may report the accepted
    /// record count required to establish their result.
    pub matched_records: usize,

    /// Fraction of total records reconstructed or inspected after pruning.
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

/// Exact existence query result paired with execution statistics.
///
/// # Runtime Role
///
/// `QueryExistenceReport` exposes whether the exact result set is non-empty.
/// The associated statistics describe the work performed by the short-circuit
/// existence path. `inspected_records` counts the candidate rows evaluated
/// before the result was established.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryExistenceReport {
    /// Whether the exact query result set is non-empty.
    pub has_match: bool,

    /// Number of candidate rows inspected before the result was established.
    pub inspected_records: usize,

    /// Runtime statistics for the existence query.
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
