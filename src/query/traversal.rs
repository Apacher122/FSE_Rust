//! Metadata traversal for geometric pruning.

use crate::query::QueryRegion;
use crate::storage::FSEIndex;

/// Traverses the FSE hierarchy to identify leaf partitions that intersect a query region.
///
/// This function implements Stage I metadata pruning, effectively filtering out large
/// portions of the search space by evaluating the intersection between partition
/// bounding regions and the provided query. The traversal engine only descends into
/// subtrees that are geometrically admissible, ensuring that only relevant leaf nodes
/// are retained for fine-grained evaluation. Formally, this executes the pruning
/// operator $\Pi(Q, P_k)$, where a partition $P_k$ is retained only if
/// $Q \cap B_k \neq \emptyset$.
///
/// # Panics
///
/// Panics if the dimensionality of the query region does not match the dimensionality
/// of the global FSE index.
pub fn traverse(index: &FSEIndex, query: &QueryRegion) -> Vec<usize> {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let query_bounds = query.as_bounds();
    let mut retained = Vec::new();

    // using a heap-allocated Vec for the traversal stack.
    // Since `max_depth` is capped at 32 in the builder, this won't blow up
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
