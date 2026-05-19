//! Metadata traversal for geometric pruning.

use crate::math::Scalar;
use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::storage::{FSEIndex, LeafReconstructionShape};

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
///
/// Normal traversal also carries cached reconstruction shape metadata so Stage
/// II does not need to look that shape back up by node id. Test constructors
/// still support id-only retained leaves for older unit tests.
#[derive(Clone, Copy, Debug)]
pub struct RetainedLeaf {
    /// Retained leaf node identifier.
    pub node_id: usize,

    /// Coverage classification for this retained leaf.
    pub coverage: RetainedLeafCoverage,

    cached_shape: Option<LeafReconstructionShape>,
}

impl PartialEq for RetainedLeaf {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.coverage == other.coverage
    }
}

impl Eq for RetainedLeaf {}

impl RetainedLeaf {
    /// Creates a retained leaf classified as covered.
    ///
    /// # Runtime Role
    ///
    /// This id-only constructor is kept for tests and compatibility helpers.
    /// Runtime traversal should prefer [`RetainedLeaf::covered_with_shape`].
    pub fn covered(node_id: usize) -> Self {
        Self {
            node_id,
            coverage: RetainedLeafCoverage::Covered,
            cached_shape: None,
        }
    }

    /// Creates a retained leaf classified as partially covered.
    ///
    /// # Runtime Role
    ///
    /// This id-only constructor is kept for tests and compatibility helpers.
    /// Runtime traversal should prefer [`RetainedLeaf::partial_with_shape`].
    pub fn partial(node_id: usize) -> Self {
        Self {
            node_id,
            coverage: RetainedLeafCoverage::Partial,
            cached_shape: None,
        }
    }

    /// Creates a covered retained leaf with cached reconstruction shape.
    pub(crate) fn covered_with_shape(shape: LeafReconstructionShape) -> Self {
        Self::with_shape(shape.node_id, RetainedLeafCoverage::Covered, shape)
    }

    /// Creates a partially covered retained leaf with cached reconstruction shape.
    ///
    /// # Runtime Role
    ///
    /// This is currently only needed by test compatibility helpers that classify
    /// retained leaf ids after traversal.
    #[cfg(test)]
    pub(crate) fn partial_with_shape(shape: LeafReconstructionShape) -> Self {
        Self::with_shape(shape.node_id, RetainedLeafCoverage::Partial, shape)
    }

    /// Creates a retained leaf with explicit coverage and cached shape.
    pub(crate) fn with_shape(
        node_id: usize,
        coverage: RetainedLeafCoverage,
        shape: LeafReconstructionShape,
    ) -> Self {
        debug_assert_eq!(
            node_id, shape.node_id,
            "retained leaf node id should match cached shape node id"
        );

        Self {
            node_id,
            coverage,
            cached_shape: Some(shape),
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

    /// Returns cached reconstruction shape for this retained leaf.
    ///
    /// # Runtime Role
    ///
    /// Traversal-produced retained leaves already carry shape metadata. Older
    /// tests and compatibility helpers may construct id-only retained leaves, so
    /// this method falls back to the index cache when needed.
    pub(crate) fn reconstruction_shape(&self, index: &FSEIndex) -> LeafReconstructionShape {
        self.cached_shape
            .unwrap_or_else(|| index.leaf_reconstruction_shape(self.node_id))
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

/// Internal traversal stack frame.
///
/// # Runtime Role
///
/// A frame carries the node id and whether the node is inside a subtree already
/// proven to be fully covered by the query. Covered descendants do not need
/// another bounds classification pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TraversalFrame {
    node_id: usize,
    inherited_covered: bool,
}

impl TraversalFrame {
    #[inline]
    fn normal(node_id: usize) -> Self {
        Self {
            node_id,
            inherited_covered: false,
        }
    }

    #[inline]
    fn covered(node_id: usize) -> Self {
        Self {
            node_id,
            inherited_covered: true,
        }
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

    let root_classification = query.classify_bounds(&index.root_node().bounds);

    traverse_with_known_root_classification(index, query, root_classification)
}

/// Traverses the FSE hierarchy using an already-known root classification.
///
/// # Runtime Role
///
/// Query execution classifies the root once to decide between full-root,
/// root-disjoint, and normal partial execution. This helper lets the normal
/// partial path reuse that classification instead of repeating root bounds
/// math inside traversal.
///
/// Public traversal validates dimensions before entering this helper. The full
/// query execution API also validates dimensions before root classification, so
/// this helper keeps only a debug assertion to catch internal misuse.
pub(crate) fn traverse_with_known_root_classification(
    index: &FSEIndex,
    query: &QueryRegion,
    root_classification: QueryBoundsClassification,
) -> QueryTraversalReport {
    debug_assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality should already be validated by the caller"
    );

    let total_leaves = index.leaf_count();

    let mut stats = QueryTraversalStats {
        total_leaves,
        ..QueryTraversalStats::default()
    };

    let mut retained_leaves = Vec::with_capacity(retained_leaf_capacity(total_leaves));
    let mut stack = Vec::with_capacity(traversal_stack_capacity(index));

    stats.visited_nodes += 1;

    match root_classification {
        QueryBoundsClassification::Covered => {
            retain_or_descend_covered_node(
                index,
                index.root,
                &mut retained_leaves,
                &mut stats,
                &mut stack,
            );
        }
        QueryBoundsClassification::Partial => {
            let root = index.root_node();

            if root.is_leaf {
                retain_leaf(
                    index.leaf_reconstruction_shape(index.root),
                    RetainedLeafCoverage::Partial,
                    &mut retained_leaves,
                    &mut stats,
                );
            } else {
                push_child_frames(&root.children, false, &mut stack);
            }
        }
        QueryBoundsClassification::Disjoint => {
            // root prune
        }
    }

    // one stack handles normal traversal and covered-subtree collection
    while let Some(frame) = stack.pop() {
        stats.visited_nodes += 1;

        let node = &index.nodes[frame.node_id];

        if frame.inherited_covered {
            retain_or_descend_covered_node(
                index,
                frame.node_id,
                &mut retained_leaves,
                &mut stats,
                &mut stack,
            );
            continue;
        }

        match query.classify_bounds(&node.bounds) {
            QueryBoundsClassification::Covered => {
                retain_or_descend_covered_node(
                    index,
                    frame.node_id,
                    &mut retained_leaves,
                    &mut stats,
                    &mut stack,
                );
            }
            QueryBoundsClassification::Partial => {
                if node.is_leaf {
                    retain_leaf(
                        index.leaf_reconstruction_shape(frame.node_id),
                        RetainedLeafCoverage::Partial,
                        &mut retained_leaves,
                        &mut stats,
                    );
                } else {
                    push_child_frames(&node.children, false, &mut stack);
                }
            }
            QueryBoundsClassification::Disjoint => {
                // safe prune
            }
        }
    }

    finish_traversal_report(retained_leaves, stats)
}

/// Returns initial capacity for retained leaves.
///
/// # Runtime Role
///
/// Most benchmark runs retain a small number of leaves, but full or broad
/// queries can retain them all. This gives small indexes enough room without
/// forcing a huge allocation for larger future indexes.
#[inline]
fn retained_leaf_capacity(total_leaves: usize) -> usize {
    total_leaves.min(64)
}

/// Returns initial capacity for the traversal stack.
///
/// # Runtime Role
///
/// The stack may grow as traversal descends, especially with smaller leaf
/// policies. A modest preallocation avoids repeated tiny reallocations while
/// keeping the buffer bounded for larger indexes.
#[inline]
fn traversal_stack_capacity(index: &FSEIndex) -> usize {
    index.node_count().min(64).max(1)
}

#[inline]
fn retained_leaf_ratio(retained_leaves: usize, total_leaves: usize) -> Scalar {
    if total_leaves == 0 {
        0.0
    } else {
        retained_leaves as Scalar / total_leaves as Scalar
    }
}

#[inline]
fn retain_or_descend_covered_node(
    index: &FSEIndex,
    node_id: usize,
    retained_leaves: &mut Vec<RetainedLeaf>,
    stats: &mut QueryTraversalStats,
    stack: &mut Vec<TraversalFrame>,
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

#[inline]
fn push_child_frames(children: &[usize], inherited_covered: bool, stack: &mut Vec<TraversalFrame>) {
    // preserve current left to right traversal order
    for child in children.iter().rev() {
        let frame = if inherited_covered {
            TraversalFrame::covered(*child)
        } else {
            TraversalFrame::normal(*child)
        };

        stack.push(frame);
    }
}

#[inline]
fn retain_leaf(
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    retained_leaves: &mut Vec<RetainedLeaf>,
    stats: &mut QueryTraversalStats,
) {
    stats.retained_leaves += 1;
    stats.retained_candidate_records += shape.cardinality;
    retained_leaves.push(RetainedLeaf::with_shape(shape.node_id, coverage, shape));
}

fn finish_traversal_report(
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
