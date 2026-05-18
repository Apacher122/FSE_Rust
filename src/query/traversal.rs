//! Metadata traversal for geometric pruning.

use crate::math::Scalar;
use crate::query::QueryRegion;
use crate::storage::FSEIndex;

/// Coverage classification for a retained leaf.
///
/// # Runtime Role
///
/// `RetainedLeafCoverage` describes whether traversal proved that a retained
/// leaf is fully covered by the query or only partially overlaps the query.
///
/// Covered leaves can skip exact per-row predicate checks during retained-leaf
/// execution. Partial leaves must still use the exact predicate path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainedLeafCoverage {
    /// The query fully contains the retained leaf bounds.
    Covered,

    /// The query intersects the retained leaf bounds but does not fully contain them.
    Partial,
}

/// A retained leaf paired with its traversal-time coverage classification.
///
/// # Runtime Role
///
/// `RetainedLeaf` is the Stage I handoff to retained-partition execution. It
/// keeps the leaf identifier together with the geometric proof discovered during
/// traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainedLeaf {
    /// Retained leaf node identifier.
    pub node_id: usize,

    /// Coverage classification for this retained leaf.
    pub coverage: RetainedLeafCoverage,
}

impl RetainedLeaf {
    /// Creates a retained leaf classified as covered.
    pub fn covered(node_id: usize) -> Self {
        Self {
            node_id,
            coverage: RetainedLeafCoverage::Covered,
        }
    }

    /// Creates a retained leaf classified as partially covered.
    pub fn partial(node_id: usize) -> Self {
        Self {
            node_id,
            coverage: RetainedLeafCoverage::Partial,
        }
    }

    /// Returns true when traversal proved full query coverage.
    pub fn is_covered(&self) -> bool {
        matches!(self.coverage, RetainedLeafCoverage::Covered)
    }

    /// Returns true when the retained leaf still requires exact predicate checks.
    pub fn is_partial(&self) -> bool {
        matches!(self.coverage, RetainedLeafCoverage::Partial)
    }
}

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
    /// production traversal report avoids storing duplicate retained-leaf state.
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
    traverse_with_stats(index, query).retained_leaf_ids()
}

/// Traverses the FSE hierarchy and returns retained leaves with traversal stats.
///
/// # Runtime Role
///
/// This function keeps traversal accounting inside the traversal stage instead
/// of mixing it with reconstruction or exact evaluation. The returned retained
/// leaves are the only partitions that later stages should reconstruct.
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

    let mut retained_leaves = Vec::new();
    let mut stack = vec![index.root];

    // tiny vec stack is fine until traversal itself shows up hot
    while let Some(node_id) = stack.pop() {
        stats.visited_nodes += 1;

        let node = &index.nodes[node_id];

        if query.contains_bounds(&node.bounds) {
            if node.is_leaf {
                retain_leaf(
                    node_id,
                    RetainedLeafCoverage::Covered,
                    &mut retained_leaves,
                    &mut stats,
                );
            } else {
                // covered subtree means no more intersection checks below here
                collect_covered_descendant_leaves(
                    index,
                    &node.children,
                    &mut retained_leaves,
                    &mut stats,
                );
            }

            continue;
        }

        if !query.intersects_bounds(&node.bounds) {
            continue;
        }

        if node.is_leaf {
            retain_leaf(
                node_id,
                RetainedLeafCoverage::Partial,
                &mut retained_leaves,
                &mut stats,
            );
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

fn retain_leaf(
    node_id: usize,
    coverage: RetainedLeafCoverage,
    retained_leaves: &mut Vec<RetainedLeaf>,
    stats: &mut QueryTraversalStats,
) {
    stats.retained_leaves += 1;
    retained_leaves.push(RetainedLeaf { node_id, coverage });
}

fn collect_covered_descendant_leaves(
    index: &FSEIndex,
    children: &[usize],
    retained_leaves: &mut Vec<RetainedLeaf>,
    stats: &mut QueryTraversalStats,
) {
    let mut stack: Vec<usize> = children.iter().rev().copied().collect();

    // covered subtree still walks ids but skips all the bounds math
    while let Some(node_id) = stack.pop() {
        stats.visited_nodes += 1;

        let node = &index.nodes[node_id];

        if node.is_leaf {
            retain_leaf(
                node_id,
                RetainedLeafCoverage::Covered,
                retained_leaves,
                stats,
            );
        } else {
            for child in node.children.iter().rev() {
                stack.push(*child);
            }
        }
    }
}
