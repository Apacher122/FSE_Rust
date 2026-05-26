//! Traversal execution algorithm.

use crate::math::Scalar;
use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::storage::{FSEIndex, LeafReconstructionShape};

use super::report::{QueryTraversalReport, QueryTraversalStats};
use super::retained_leaf::{RetainedLeaf, RetainedLeafCoverage};
use super::stack::{TraversalFrame, TraversalStack};

const DEFAULT_RETAINED_LEAF_CAPACITY: usize = 4;

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
/// Public traversal and the full query execution API already validate
/// dimensionality before entering this helper. The release assertion is kept
/// here anyway because this helper is still the boundary where retained leaves
/// are produced from geometry. Keeping the guard is safer than relying on every
/// future internal caller to preserve that precondition.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub(crate) fn traverse_with_known_root_classification(
    index: &FSEIndex,
    query: &QueryRegion,
    root_classification: QueryBoundsClassification,
) -> QueryTraversalReport {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    let total_leaves = index.leaf_count();

    let mut stats = QueryTraversalStats {
        total_leaves,
        ..QueryTraversalStats::default()
    };

    let mut retained_leaves = Vec::with_capacity(retained_leaf_capacity(total_leaves));
    let mut stack = TraversalStack::new();

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
/// Small selective queries usually retain only a couple of leaves. Starting
/// with a small buffer avoids overallocating for those hot paths while still
/// allowing broad partial queries to grow normally.
#[inline]
fn retained_leaf_capacity(total_leaves: usize) -> usize {
    total_leaves.min(DEFAULT_RETAINED_LEAF_CAPACITY)
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

#[inline]
fn push_child_frames(children: &[usize], inherited_covered: bool, stack: &mut TraversalStack) {
    match children.len() {
        0 => {}
        1 => {
            stack.push(child_frame(children[0], inherited_covered));
        }
        2 => {
            // preserve left to right pop order without the iterator path
            stack.push(child_frame(children[1], inherited_covered));
            stack.push(child_frame(children[0], inherited_covered));
        }
        _ => {
            // keep the generic fallback in case future splitters use wider fanout
            for child in children.iter().rev() {
                stack.push(child_frame(*child, inherited_covered));
            }
        }
    }
}

#[inline]
fn child_frame(node_id: usize, inherited_covered: bool) -> TraversalFrame {
    if inherited_covered {
        TraversalFrame::covered(node_id)
    } else {
        TraversalFrame::normal(node_id)
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
