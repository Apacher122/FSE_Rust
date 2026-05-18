use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::execution::{
    execute_fully_covered_index_serial, execute_fully_covered_index_with_options,
    fully_covered_retained_leaves, leaf_count,
};
use crate::query::{
    QueryExecutionMode, QueryExecutionOptions, QueryRegion, RetainedLeaf, execute_query_with_stats,
    execute_query_with_stats_and_options,
};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn fully_covered_retained_leaves_returns_every_leaf_as_covered() {
    let index = two_leaf_index();

    let retained_leaves = fully_covered_retained_leaves(&index);

    assert_eq!(
        retained_leaves,
        vec![RetainedLeaf::covered(1), RetainedLeaf::covered(2)]
    );
}

#[test]
fn leaf_count_counts_only_leaf_nodes() {
    let index = two_leaf_index();

    assert_eq!(leaf_count(&index), 2);
}

#[test]
fn fully_covered_index_serial_reconstructs_all_leaf_rows_in_index_order() {
    let index = two_leaf_index();

    let report = execute_fully_covered_index_serial(&index);

    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![8.0, 8.0]),
            Vector::new(vec![9.0, 9.0]),
        ]
    );
    assert_eq!(report.reconstructed_records, 4);
    assert_eq!(report.predicate_evaluated_records, 0);
    assert_eq!(report.matched_records, 4);
}

#[test]
fn execute_query_with_stats_uses_root_coverage_fast_path() {
    let index = two_leaf_index();
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);

    let report = execute_query_with_stats(&index, &query);

    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![8.0, 8.0]),
            Vector::new(vec![9.0, 9.0]),
        ]
    );
    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.total_leaves, 2);
    assert_eq!(report.stats.retained_leaves, 2);
    assert_eq!(report.stats.retained_leaf_ratio, 1.0);
    assert_eq!(report.stats.total_records, 4);
    assert_eq!(report.stats.reconstructed_records, 4);
    assert_eq!(report.stats.matched_records, 4);
    assert_eq!(report.stats.candidate_ratio, 1.0);
}

#[test]
fn fully_covered_index_with_options_preserves_parallel_execution_option() {
    let index = two_leaf_index();
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let options = QueryExecutionOptions {
        mode: QueryExecutionMode::Parallel,
        parallel_min_retained_leaves: 1,
    };

    let report = execute_fully_covered_index_with_options(&index, &query, options);

    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.retained_leaves, 2);
    assert_eq!(report.stats.reconstructed_records, 4);
    assert_eq!(report.stats.matched_records, 4);
    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![8.0, 8.0]),
            Vector::new(vec![9.0, 9.0]),
        ]
    );
}

#[test]
fn execute_query_with_stats_uses_normal_traversal_when_root_is_not_covered() {
    let index = two_leaf_index();
    let query = QueryRegion::new(vec![0.0, 0.0], vec![5.0, 5.0]);

    let report =
        execute_query_with_stats_and_options(&index, &query, QueryExecutionOptions::serial());

    assert_eq!(report.stats.visited_nodes, 3);
    assert_eq!(report.stats.total_leaves, 2);
    assert_eq!(report.stats.retained_leaves, 1);
    assert_eq!(report.stats.reconstructed_records, 2);
    assert_eq!(report.stats.matched_records, 2);
    assert_eq!(
        report.results,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0])]
    );
}

fn two_leaf_index() -> FSEIndex {
    let root = PartitionNode::with_cardinality(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        4,
        vec![1, 2],
        false,
    );

    let left_child = PartitionNode::from_points(
        1,
        &[Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0])],
    );

    let right_child = PartitionNode::from_points(
        2,
        &[Vector::new(vec![8.0, 8.0]), Vector::new(vec![9.0, 9.0])],
    );

    FSEIndex::new(vec![root, left_child, right_child], 0)
}
