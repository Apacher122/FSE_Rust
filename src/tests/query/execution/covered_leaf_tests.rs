use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::execution::{
    execute_covered_retained_leaf, execute_partially_covered_retained_leaf,
    execute_query_with_stats, execute_retained_leaf, execute_retained_leaves,
    query_contains_bounds,
};
use crate::query::{QueryRegion, execute_query};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn query_contains_bounds_returns_true_for_fully_contained_bounds() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![2.0, 3.0], vec![8.0, 9.0]);

    assert!(query_contains_bounds(&query, &bounds));
}

#[test]
fn query_contains_bounds_returns_true_for_equal_bounds() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]);

    assert!(query_contains_bounds(&query, &bounds));
}

#[test]
fn query_contains_bounds_returns_false_for_partial_overlap() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![5.0, 5.0], vec![12.0, 9.0]);

    assert!(!query_contains_bounds(&query, &bounds));
}

#[test]
fn query_contains_bounds_returns_false_when_bounds_extend_below_query() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![-1.0, 2.0], vec![8.0, 9.0]);

    assert!(!query_contains_bounds(&query, &bounds));
}

#[test]
fn covered_retained_leaf_reconstructs_all_rows_without_predicate_evaluation() {
    let node = covered_leaf_node();

    let report = execute_covered_retained_leaf(&node, 2);

    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![3.0, 3.0]),
        ]
    );
    assert_eq!(report.reconstructed_records, 3);
    assert_eq!(report.predicate_evaluated_records, 0);
    assert_eq!(report.matched_records, 3);
}

#[test]
fn partially_covered_retained_leaf_preserves_exact_predicate_evaluation() {
    let node = covered_leaf_node();
    let query = QueryRegion::new(vec![2.0, 2.0], vec![3.0, 3.0]);

    let report = execute_partially_covered_retained_leaf(&node, &query, 2);

    assert_eq!(
        report.results,
        vec![Vector::new(vec![2.0, 2.0]), Vector::new(vec![3.0, 3.0])]
    );
    assert_eq!(report.reconstructed_records, 3);
    assert_eq!(report.predicate_evaluated_records, 3);
    assert_eq!(report.matched_records, 2);
}

#[test]
fn execute_retained_leaf_uses_covered_fast_path_when_query_contains_leaf_bounds() {
    let node = covered_leaf_node();
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);

    let report = execute_retained_leaf(&node, &query, 2);

    assert_eq!(report.results.len(), 3);
    assert_eq!(report.reconstructed_records, 3);
    assert_eq!(report.predicate_evaluated_records, 0);
    assert_eq!(report.matched_records, 3);
}

#[test]
fn execute_retained_leaf_uses_exact_path_when_query_partially_overlaps_leaf_bounds() {
    let node = covered_leaf_node();
    let query = QueryRegion::new(vec![2.0, 2.0], vec![10.0, 10.0]);

    let report = execute_retained_leaf(&node, &query, 2);

    assert_eq!(
        report.results,
        vec![Vector::new(vec![2.0, 2.0]), Vector::new(vec![3.0, 3.0])]
    );
    assert_eq!(report.reconstructed_records, 3);
    assert_eq!(report.predicate_evaluated_records, 3);
    assert_eq!(report.matched_records, 2);
}

#[test]
fn retained_leaf_batch_tracks_predicate_evaluation_savings() {
    let index = covered_and_partial_index();
    let query = QueryRegion::new(vec![0.0, 0.0], vec![3.0, 3.0]);

    let report = execute_retained_leaves(&index, &query, &[1, 2]);

    assert_eq!(report.reconstructed_records, 6);
    assert_eq!(report.predicate_evaluated_records, 3);
    assert_eq!(report.matched_records, 5);
    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![3.0, 3.0]),
            Vector::new(vec![3.0, 1.0]),
        ]
    );
}

#[test]
fn execute_query_preserves_exact_results_with_covered_leaf_fast_path() {
    let index = covered_and_partial_index();
    let query = QueryRegion::new(vec![0.0, 0.0], vec![3.0, 3.0]);

    let results = execute_query(&index, &query);

    assert_eq!(
        results,
        vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![3.0, 3.0]),
            Vector::new(vec![3.0, 1.0]),
        ]
    );
}

#[test]
fn execute_query_with_stats_preserves_existing_public_stats() {
    let index = covered_and_partial_index();
    let query = QueryRegion::new(vec![0.0, 0.0], vec![3.0, 3.0]);

    let report = execute_query_with_stats(&index, &query);

    assert_eq!(report.stats.visited_nodes, 3);
    assert_eq!(report.stats.total_leaves, 2);
    assert_eq!(report.stats.retained_leaves, 2);
    assert_eq!(report.stats.total_records, 6);
    assert_eq!(report.stats.reconstructed_records, 6);
    assert_eq!(report.stats.matched_records, 5);
    assert_eq!(report.results.len(), 5);
}

fn covered_leaf_node() -> PartitionNode {
    PartitionNode::new(
        0,
        vec![2.0, 2.0],
        BoundingBox::new(vec![1.0, 1.0], vec![3.0, 3.0]),
        ResidualBlock::new(vec![vec![-1.0, 0.0, 1.0], vec![-1.0, 0.0, 1.0]]),
        Vec::new(),
        true,
    )
}

fn covered_and_partial_index() -> FSEIndex {
    let root = PartitionNode::with_cardinality(
        0,
        vec![2.5, 2.5],
        BoundingBox::new(vec![0.0, 0.0], vec![6.0, 6.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        6,
        vec![1, 2],
        false,
    );

    let covered_child = PartitionNode::new(
        1,
        vec![1.0, 1.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![-1.0, 0.0, 1.0], vec![-1.0, 0.0, 1.0]]),
        Vec::new(),
        true,
    );

    let partial_child = PartitionNode::new(
        2,
        vec![4.0, 3.0],
        BoundingBox::new(vec![3.0, 1.0], vec![6.0, 6.0]),
        ResidualBlock::new(vec![vec![-1.0, -1.0, 2.0], vec![0.0, -2.0, 3.0]]),
        Vec::new(),
        true,
    );

    FSEIndex::new(vec![root, covered_child, partial_child], 0)
}
