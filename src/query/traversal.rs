//! Metadata traversal for geometric pruning.

use crate::math::Scalar;
use crate::query::QueryRegion;
use crate::storage::FSEIndex;

/// Runtime statistics collected during geometric traversal.
///
/// # Runtime Role
///
/// `QueryTraversalStats` describes how much hierarchy metadata was inspected
/// during Stage I of query execution. These values are independent from
/// reconstruction and exact predicate evaluation.
///
/// # Formal Reference
///
/// These values correspond to the geometric selection stage of the FSE
/// execution pipeline, where traversal visits hierarchy nodes and retains leaf
/// partitions whose bounding regions intersect the query region.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueryTraversalStats {
    /// Number of hierarchy nodes whose metadata was visited.
    pub visited_nodes: usize,

    /// Number of leaf partitions in the index.
    pub total_leaves: usize,

    /// Number of leaf partitions retained after metadata pruning.
    pub retained_leaves: usize,

    /// Fraction of leaf partitions retained after metadata pruning.
    pub retained_leaf_ratio: Scalar,
}

/// Retained leaf identifiers paired with traversal statistics.
///
/// # Runtime Role
///
/// `QueryTraversalReport` is the output of Stage I query execution. It records
/// the leaf partitions that are geometrically admissible and the amount of
/// metadata work required to discover them.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryTraversalReport {
    /// Leaf node identifiers retained by geometric pruning.
    pub retained_leaf_ids: Vec<usize>,

    /// Traversal statistics collected while walking the hierarchy.
    pub stats: QueryTraversalStats,
}

/// Traverses the FSE hierarchy and returns retained leaf partitions.
///
/// # Runtime Role
///
/// Traversal performs Stage I metadata pruning. It evaluates partition bounding
/// regions against the query region and descends only into geometrically
/// admissible subtrees.
///
/// # Formal Reference
///
/// This implements the pruning operator `Pi(Q, P_k)`, where a partition is
/// retained when `Q intersect B_k` is non-empty.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn traverse(index: &FSEIndex, query: &QueryRegion) -> Vec<usize> {
    traverse_with_stats(index, query).retained_leaf_ids
}

/// Traverses the FSE hierarchy and returns retained leaves with traversal stats.
///
/// # Runtime Role
///
/// This function keeps traversal accounting inside the traversal stage instead
/// of mixing it with reconstruction or exact evaluation. The returned leaf IDs
/// are the only partitions that later stages should reconstruct.
///
/// # Formal Reference
///
/// This realizes Stage I of the execution pipeline:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// Only the geometric stage is performed here.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn traverse_with_stats(index: &FSEIndex, query: &QueryRegion) -> QueryTraversalReport {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let total_leaves = index.nodes.iter().filter(|node| node.is_leaf).count();

    let mut stats = QueryTraversalStats {
        total_leaves,
        ..QueryTraversalStats::default()
    };

    let query_bounds = query.as_bounds();
    let mut retained_leaf_ids = Vec::new();
    let mut stack = vec![index.root];

    // tiny vec stack is fine until traversal itself shows up hot
    while let Some(node_id) = stack.pop() {
        stats.visited_nodes += 1;

        let node = &index.nodes[node_id];

        if !node.bounds.intersects(&query_bounds) {
            continue;
        }

        if node.is_leaf {
            stats.retained_leaves += 1;
            retained_leaf_ids.push(node_id);
        } else {
            // preserve current left to right traversal order
            for child in node.children.iter().rev() {
                stack.push(*child);
            }
        }
    }

    stats.retained_leaf_ratio = if stats.total_leaves == 0 {
        0.0
    } else {
        stats.retained_leaves as Scalar / stats.total_leaves as Scalar
    };

    QueryTraversalReport {
        retained_leaf_ids,
        stats,
    }
}
