//! Metadata traversal for geometric pruning.

use crate::query::QueryRegion;
use crate::storage::FSEIndex;

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
