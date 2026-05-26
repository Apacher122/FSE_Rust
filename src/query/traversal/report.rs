//! Traversal report types.

use crate::math::Scalar;

use super::retained_leaf::RetainedLeaf;

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

    /// Number of records contained by the retained leaves.
    ///
    /// # Runtime Role
    ///
    /// This is the candidate reconstruction count discovered during traversal.
    /// Carrying it forward avoids a second pass over retained leaves before
    /// serial execution can allocate its result buffer.
    pub retained_candidate_records: usize,

    /// Fraction of leaf partitions retained after metadata pruning.
    pub retained_leaf_ratio: Scalar,
}

/// Retained leaves paired with traversal statistics.
///
/// # Runtime Role
///
/// `QueryTraversalReport` is the output of Stage I query execution. It records
/// the leaf partitions that are geometrically admissible and the amount of
/// metadata work required to discover them.
///
/// Production query execution consumes the classified retained leaves directly.
/// Legacy callers that only need node identifiers can derive them with
/// [`QueryTraversalReport::retained_leaf_ids`].
#[derive(Clone, Debug, PartialEq)]
pub struct QueryTraversalReport {
    /// Leaf node identifiers retained by geometric pruning.
    ///
    /// # Runtime Role
    ///
    /// This test-only field keeps the existing unit tests stable while the
    /// production query path avoids storing duplicate retained-leaf state.
    #[cfg(test)]
    pub retained_leaf_ids: Vec<usize>,

    /// Retained leaves with traversal-time coverage classification.
    pub retained_leaves: Vec<RetainedLeaf>,

    /// Traversal statistics collected while walking the hierarchy.
    pub stats: QueryTraversalStats,
}

impl QueryTraversalReport {
    /// Returns retained leaf node identifiers in traversal order.
    ///
    /// # Runtime Role
    ///
    /// This preserves the original id-only traversal shape without forcing the
    /// production query path to store a second retained-leaf vector.
    pub fn retained_leaf_ids(&self) -> Vec<usize> {
        self.retained_leaves
            .iter()
            .map(|retained_leaf| retained_leaf.node_id)
            .collect()
    }
}
