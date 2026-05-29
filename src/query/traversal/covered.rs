//! Covered-subtree traversal handling.

use crate::storage::FSEIndex;

use super::report::QueryTraversalStats;
use super::retained_leaf::{RetainedLeaf, RetainedLeafCoverage};
use super::retention::retain_leaf;
use super::stack::{TraversalStack, push_child_frames};

/// Retains a covered leaf or descends through a covered internal subtree.
///
/// # Runtime Role
///
/// When traversal proves that a node is covered by the query, descendants do not
/// need additional bounds classification. Leaf nodes are retained as covered.
/// Internal nodes push covered child frames so the traversal stack preserves
/// normal left-to-right leaf order.
///
/// # Formal Reference
///
/// This applies the covered branch of the geometric pruning rule, where
/// $B_k \subseteq Q$ allows all descendant leaves to skip exact metadata
/// intersection checks.
#[inline]
pub(super) fn retain_or_descend_covered_node(
    index: &FSEIndex,
    node_id: usize,
    retained_leaves: &mut Vec<RetainedLeaf>,
    stats: &mut QueryTraversalStats,
    stack: &mut TraversalStack,
) {
    let node = &index.nodes[node_id];

    if node.is_leaf {
        retain_leaf(
            index.leaf_reconstruction_shape(node_id),
            RetainedLeafCoverage::Covered,
            retained_leaves,
            stats,
        );
    } else {
        // covered subtree means no more bounds math below this point
        push_child_frames(&node.children, true, stack);
    }
}
