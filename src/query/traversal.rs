//! Metadata traversal for geometric pruning.

use crate::query::QueryRegion;
use crate::storage::FSEIndex;

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
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let query_bounds = query.as_bounds();
    let mut retained = Vec::new();
    let mut stack = vec![index.root];

    while let Some(node_id) = stack.pop() {
        let node = &index.nodes[node_id];

        if !node.bounds.intersects(&query_bounds) {
            continue;
        }

        if node.is_leaf {
            retained.push(node_id);
        } else {
            for child in node.children.iter().rev() {
                stack.push(*child);
            }
        }
    }
    retained
}
