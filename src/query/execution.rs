//! End-to-end query execution.

use crate::math::Vector;
use crate::query::{QueryRegion, evaluate_query, reconstruct_partition};
use crate::storage::FSEIndex;

/// Runtime statistics collected during query execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryExecutionStats {
    pub visited_nodes: usize,
    pub retained_leaves: usize,
    pub reconstructed_records: usize,
    pub matched_records: usize,
}

/// Query result paired with execution statistics.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryExecutionReport {
    pub results: Vec<Vector>,
    pub stats: QueryExecutionStats,
}

/// Executes a query and returns exact matches with execution statistics.
pub fn execute_query_with_stats(index: &FSEIndex, query: &QueryRegion) -> QueryExecutionReport {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let mut stats = QueryExecutionStats::default();
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

    QueryExecutionReport { results, stats }
}

pub fn execute_query(index: &FSEIndex, query: &QueryRegion) -> Vec<Vector> {
    execute_query_with_stats(index, query).results
}
