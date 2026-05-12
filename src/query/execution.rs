//! End-to-end query execution.

use crate::math::{Scalar, Vector};
use crate::query::{QueryRegion, evaluate_query, reconstruct_partition};
use crate::storage::FSEIndex;

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
    /// Number of leaf partitions retained after metadata pruning.
    pub retained_leaves: usize,
    /// Number of records represented by the index.
    pub total_records: usize,
    /// Number of records reconstructed after pruning.
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

/// Executes a query against an FSE index.
///
/// # Runtime Role
///
/// This function composes the complete minimal query pipeline:
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
    execute_query_with_stats(index, query).results
}

/// Executes a query and returns exact matches with execution statistics.
///
/// # Runtime Role
///
/// This provides an instrumented execution path for correctness validation,
/// benchmarking, and future optimization work.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn execute_query_with_stats(index: &FSEIndex, query: &QueryRegion) -> QueryExecutionReport {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let mut stats = QueryExecutionStats {
        total_records: index.root_node().cardinality,
        ..QueryExecutionStats::default()
    };

    let mut results = Vec::new();

    let query_bounds = query.as_bounds();
    let mut stack = vec![index.root];

    while let Some(node_id) = stack.pop() {
        stats.visited_nodes += 1;
        let node = &index.nodes[node_id];

        if !node.bounds.intersects(&query_bounds) {
            continue;
        }

        if node.is_leaf {
            stats.retained_leaves += 1;
            let reconstructed = reconstruct_partition(node);
            stats.reconstructed_records += reconstructed.len();

            let matches = evaluate_query(&reconstructed, query);
            stats.matched_records += matches.len();

            results.extend(matches);
        } else {
            for child in node.children.iter().rev() {
                stack.push(*child);
            }
        }
    }

    stats.candidate_ratio = if stats.total_records == 0 {
        0.0
    } else {
        stats.reconstructed_records as Scalar / stats.total_records as Scalar
    };

    QueryExecutionReport { results, stats }
}
