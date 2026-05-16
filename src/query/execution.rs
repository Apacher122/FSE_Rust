//! End-to-end query execution.

use crate::math::{Scalar, Vector};
use crate::query::{QueryRegion, reconstruct_row_into};
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
/// Candidate rows are reconstructed into a reusable coordinate buffer and then
/// evaluated immediately. An owned `Vector` is allocated only when the row
/// satisfies the exact query predicate.
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
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let total_leaves = index.nodes.iter().filter(|node| node.is_leaf).count();

    let mut stats = QueryExecutionStats {
        total_leaves,
        total_records: index.root_node().cardinality,
        ..QueryExecutionStats::default()
    };

    let mut results = Vec::new();
    let mut reconstructed_values = Vec::with_capacity(index.dimensions);

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

            let row_count = node.residuals.cardinality();
            stats.reconstructed_records += row_count;

            // this is the point of the commit
            // dont materialize a whole temp vec if the final query only keeps a few rows
            for row in 0..row_count {
                reconstruct_row_into(node, row, &mut reconstructed_values);

                if query.contains_values(&reconstructed_values) {
                    stats.matched_records += 1;
                    results.push(Vector::new(reconstructed_values.clone()));
                }
            }
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

    stats.retained_leaf_ratio = if stats.total_leaves == 0 {
        0.0
    } else {
        stats.retained_leaves as Scalar / stats.total_leaves as Scalar
    };

    QueryExecutionReport { results, stats }
}
