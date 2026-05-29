//! Exact existence query execution.
//!
//! Existence queries answer whether at least one record satisfies the query
//! without materializing owned result rows or computing the full cardinality.

use crate::query::region::QueryBoundsClassification;
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::root::classify_query_root;
use crate::query::traversal::traverse_with_known_root_classification;

/// Returns true when at least one indexed record satisfies the query.
///
/// # Runtime Role
///
/// This output contract is useful when callers need exact existence rather than
/// materialized rows or exact cardinality. It preserves the same query semantics
/// as owned-result and count-only execution, but it may stop row evaluation once
/// one exact match is found.
///
/// # Formal Reference
///
/// This preserves the paper's staged execution model:
///
/// `Geometry -> Reconstruction -> Logic`
///
/// Let `E(Q, F)` denote the exact FSE execution result:
///
/// `E(Q, F) = σ_Q(Φ(R_T(Q)))`
///
/// This function returns whether that exact result set is non-empty:
///
/// `|E(Q, F)| > 0`
///
/// Equivalently, it returns whether there exists at least one reconstructed
/// candidate record that satisfies the exact query predicate.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn query_has_match(index: &FSEIndex, query: &QueryRegion) -> bool {
    let root_classification = classify_query_root(index, query);

    match root_classification {
        QueryBoundsClassification::Covered => {
            return index.root_node().cardinality > 0;
        }
        QueryBoundsClassification::Disjoint => {
            return false;
        }
        QueryBoundsClassification::Partial => {
            // normal path reuses the root classification already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    retained_leaves_have_match(index, query, &traversal_report.retained_leaves)
}

fn retained_leaves_have_match(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
) -> bool {
    for retained_leaf in retained_leaves {
        let node = &index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(index);

        if retained_leaf_has_match(node, shape, retained_leaf.coverage, query) {
            return true;
        }
    }

    false
}

fn retained_leaf_has_match(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    query: &QueryRegion,
) -> bool {
    match coverage {
        RetainedLeafCoverage::Covered => shape.cardinality > 0,
        RetainedLeafCoverage::Partial => partial_retained_leaf_has_match(node, shape, query),
    }
}

fn partial_retained_leaf_has_match(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> bool {
    match shape.dimensions {
        1 => partial_retained_leaf_has_match_1d(node, shape, query),
        2 => partial_retained_leaf_has_match_2d(node, shape, query),
        _ => partial_retained_leaf_has_match_generic(node, shape, query),
    }
}

fn partial_retained_leaf_has_match_1d(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> bool {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];

    for row in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row];

        if value_0 >= query_min_0 && value_0 <= query_max_0 {
            return true;
        }
    }

    false
}

fn partial_retained_leaf_has_match_2d(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> bool {
    let centroid_0 = node.centroid[0];
    let centroid_1 = node.centroid[1];

    let residual_0 = &node.residuals.dimensions[0];
    let residual_1 = &node.residuals.dimensions[1];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];
    let query_min_1 = query.min[1];
    let query_max_1 = query.max[1];

    for row in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row];

        if value_0 < query_min_0 || value_0 > query_max_0 {
            continue;
        }

        let value_1 = centroid_1 + residual_1[row];

        if value_1 >= query_min_1 && value_1 <= query_max_1 {
            return true;
        }
    }

    false
}

fn partial_retained_leaf_has_match_generic(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> bool {
    let mut values = vec![0.0; shape.dimensions];

    // exact existence still needs the predicate, just not every accepted row
    for row in 0..shape.cardinality {
        for dimension in 0..shape.dimensions {
            values[dimension] =
                node.centroid[dimension] + node.residuals.dimensions[dimension][row];
        }

        if query.contains_values_prevalidated(&values, shape.dimensions) {
            return true;
        }
    }

    false
}
