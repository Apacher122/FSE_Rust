//! Exact existence query execution.
//!
//! Existence queries return whether the exact result set `E(Q, F)` is non-empty.
//! The execution path preserves `Geometry -> Reconstruction -> Logic` and may
//! stop candidate evaluation after the first exact match.

use crate::query::region::QueryBoundsClassification;
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::reports::QueryExistenceReport;
use super::root::classify_query_root;
use super::stats::{
    root_covered_stats_with_counts, root_disjoint_stats, stats_from_traversal_with_counts,
};
use crate::query::traversal::traverse_with_known_root_classification;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ExistenceSearchReport {
    has_match: bool,
    inspected_records: usize,
}

impl ExistenceSearchReport {
    fn matched_records(self) -> usize {
        usize::from(self.has_match)
    }
}

/// Returns true when at least one indexed record satisfies the query.
///
/// # Runtime Role
///
/// This output contract is useful when callers need exact existence rather than
/// materialized rows, row references, or exact cardinality. It evaluates the
/// same exact result set `E(Q, F)` as the other query output contracts and
/// returns whether that set is non-empty.
///
/// # Formal Reference
///
/// This preserves the staged execution model:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// Let `E(Q, F)` denote the exact FSE execution result:
///
/// `E(Q, F) = σ_Q(Φ(R_T(Q)))`
///
/// This function returns whether that exact result set is non-empty:
///
/// `|E(Q, F)| > 0`
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn query_has_match(index: &FSEIndex, query: &QueryRegion) -> bool {
    query_has_match_with_stats(index, query).has_match
}

/// Returns exact existence with execution statistics.
///
/// # Runtime Role
///
/// The report records the boolean existence result and the amount of candidate
/// work required to establish it. For partial retained leaves, candidate records
/// are counted until the first exact match is found or all retained candidates
/// have been rejected.
///
/// `QueryExecutionStats::matched_records` is `1` when existence is proven and
/// `0` otherwise. `QueryExecutionStats::reconstructed_records` matches
/// `inspected_records` for this output contract.
///
/// # Formal Reference
///
/// The result is a boolean projection over the exact execution result:
///
/// `query_has_match(Q, F) = |E(Q, F)| > 0`
///
/// Partial geometric retention still requires exact evaluation through `σ_Q`
/// before the existence result can be established.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
pub fn query_has_match_with_stats(index: &FSEIndex, query: &QueryRegion) -> QueryExistenceReport {
    let root_classification = classify_query_root(index, query);

    match root_classification {
        QueryBoundsClassification::Covered => {
            return existence_fully_covered_index(index);
        }
        QueryBoundsClassification::Disjoint => {
            return existence_root_disjoint_query(index);
        }
        QueryBoundsClassification::Partial => {
            // normal path reuses the root classification already paid for
        }
    }

    let traversal_report =
        traverse_with_known_root_classification(index, query, root_classification);

    let search_report = retained_leaves_have_match(index, query, &traversal_report.retained_leaves);

    QueryExistenceReport {
        has_match: search_report.has_match,
        inspected_records: search_report.inspected_records,
        stats: stats_from_traversal_with_counts(
            index,
            &traversal_report,
            search_report.inspected_records,
            search_report.matched_records(),
        ),
    }
}

fn existence_fully_covered_index(index: &FSEIndex) -> QueryExistenceReport {
    let has_match = index.root_node().cardinality > 0;
    let inspected_records = usize::from(has_match);

    QueryExistenceReport {
        has_match,
        inspected_records,
        stats: root_covered_stats_with_counts(index, inspected_records, usize::from(has_match)),
    }
}

fn existence_root_disjoint_query(index: &FSEIndex) -> QueryExistenceReport {
    QueryExistenceReport {
        has_match: false,
        inspected_records: 0,
        stats: root_disjoint_stats(index),
    }
}

fn retained_leaves_have_match(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
) -> ExistenceSearchReport {
    let mut inspected_records = 0;

    for retained_leaf in retained_leaves {
        let node = &index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(index);
        let leaf_report = retained_leaf_has_match(node, shape, retained_leaf.coverage, query);

        inspected_records += leaf_report.inspected_records;

        if leaf_report.has_match {
            return ExistenceSearchReport {
                has_match: true,
                inspected_records,
            };
        }
    }

    ExistenceSearchReport {
        has_match: false,
        inspected_records,
    }
}

fn retained_leaf_has_match(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    query: &QueryRegion,
) -> ExistenceSearchReport {
    match coverage {
        RetainedLeafCoverage::Covered => ExistenceSearchReport {
            has_match: shape.cardinality > 0,
            inspected_records: usize::from(shape.cardinality > 0),
        },
        RetainedLeafCoverage::Partial => partial_retained_leaf_has_match(node, shape, query),
    }
}

fn partial_retained_leaf_has_match(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> ExistenceSearchReport {
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
) -> ExistenceSearchReport {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    let query_min_0 = query.min[0];
    let query_max_0 = query.max[0];

    for row in 0..shape.cardinality {
        let value_0 = centroid_0 + residual_0[row];

        if value_0 >= query_min_0 && value_0 <= query_max_0 {
            return ExistenceSearchReport {
                has_match: true,
                inspected_records: row + 1,
            };
        }
    }

    ExistenceSearchReport {
        has_match: false,
        inspected_records: shape.cardinality,
    }
}

fn partial_retained_leaf_has_match_2d(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> ExistenceSearchReport {
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
            return ExistenceSearchReport {
                has_match: true,
                inspected_records: row + 1,
            };
        }
    }

    ExistenceSearchReport {
        has_match: false,
        inspected_records: shape.cardinality,
    }
}

fn partial_retained_leaf_has_match_generic(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
) -> ExistenceSearchReport {
    let mut values = vec![0.0; shape.dimensions];

    // exact existence still needs the predicate just not every accepted row
    for row in 0..shape.cardinality {
        for dimension in 0..shape.dimensions {
            values[dimension] =
                node.centroid[dimension] + node.residuals.dimensions[dimension][row];
        }

        if query.contains_values_prevalidated(&values, shape.dimensions) {
            return ExistenceSearchReport {
                has_match: true,
                inspected_records: row + 1,
            };
        }
    }

    ExistenceSearchReport {
        has_match: false,
        inspected_records: shape.cardinality,
    }
}
