//! Retained-leaf traversal accounting.

use crate::math::Scalar;
use crate::storage::LeafReconstructionShape;

use super::report::{QueryTraversalReport, QueryTraversalStats};
use super::retained_leaf::{RetainedLeaf, RetainedLeafCoverage};

const DEFAULT_RETAINED_LEAF_CAPACITY: usize = 4;

/// Returns initial capacity for retained leaves.
///
/// # Runtime Role
///
/// Small selective queries usually retain only a couple of leaves. Starting
/// with a small buffer avoids overallocating for those hot paths while still
/// allowing broad partial queries to grow normally.
#[inline]
pub(super) fn retained_leaf_capacity(total_leaves: usize) -> usize {
    total_leaves.min(DEFAULT_RETAINED_LEAF_CAPACITY)
}

/// Records one retained leaf and updates traversal accounting.
///
/// # Runtime Role
///
/// Traversal produces both retained-leaf identities and the candidate record
/// count consumed by later execution stages. Keeping the accounting update next
/// to retained-leaf construction prevents those two values from drifting.
#[inline]
pub(super) fn retain_leaf(
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    retained_leaves: &mut Vec<RetainedLeaf>,
    stats: &mut QueryTraversalStats,
) {
    stats.retained_leaves += 1;
    stats.retained_candidate_records += shape.cardinality;
    retained_leaves.push(RetainedLeaf::with_shape(shape.node_id, coverage, shape));
}

/// Finalizes traversal output after the stack has been exhausted.
///
/// # Runtime Role
///
/// This computes derived traversal statistics and preserves the test-only
/// retained-leaf id view without forcing production query execution to store a
/// duplicate id vector.
pub(super) fn finish_traversal_report(
    retained_leaves: Vec<RetainedLeaf>,
    mut stats: QueryTraversalStats,
) -> QueryTraversalReport {
    stats.retained_leaf_ratio = retained_leaf_ratio(stats.retained_leaves, stats.total_leaves);

    #[cfg(test)]
    let retained_leaf_ids = retained_leaves
        .iter()
        .map(|retained_leaf| retained_leaf.node_id)
        .collect();

    QueryTraversalReport {
        #[cfg(test)]
        retained_leaf_ids,
        retained_leaves,
        stats,
    }
}

#[inline]
fn retained_leaf_ratio(retained_leaves: usize, total_leaves: usize) -> Scalar {
    if total_leaves == 0 {
        0.0
    } else {
        retained_leaves as Scalar / total_leaves as Scalar
    }
}
