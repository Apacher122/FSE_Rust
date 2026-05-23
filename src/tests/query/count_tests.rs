use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::{
    QueryRegion, count_query_matches, count_query_matches_with_stats, execute_query,
    execute_query_with_stats,
};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn count_query_matches_matches_owned_query_result_len() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);

    let count = count_query_matches(&index, &query);
    let owned_results = execute_query(&index, &query);

    assert_eq!(count, owned_results.len());
    assert_eq!(count, 2);
}

#[test]
fn count_query_matches_with_stats_reports_root_prune() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);

    let report = count_query_matches_with_stats(&index, &query);

    assert_eq!(report.matched_records, 0);
    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.total_leaves, 1);
    assert_eq!(report.stats.retained_leaves, 0);
    assert_eq!(report.stats.retained_leaf_ratio, 0.0);
    assert_eq!(report.stats.total_records, 2);
    assert_eq!(report.stats.reconstructed_records, 0);
    assert_eq!(report.stats.matched_records, 0);
    assert_eq!(report.stats.candidate_ratio, 0.0);
}

#[test]
fn count_query_matches_with_stats_matches_owned_execution_stats() {
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
        &[Vector::new(vec![0.0, 0.0]), Vector::new(vec![2.0, 2.0])],
    );

    let right_child = PartitionNode::from_points(
        2,
        &[Vector::new(vec![8.0, 8.0]), Vector::new(vec![10.0, 10.0])],
    );

    let index = FSEIndex::new(vec![root, left_child, right_child], 0);
    let query = QueryRegion::new(vec![1.0, 1.0], vec![8.0, 8.0]);

    let count_report = count_query_matches_with_stats(&index, &query);
    let owned_report = execute_query_with_stats(&index, &query);

    assert_eq!(count_report.matched_records, owned_report.results.len());
    assert_eq!(count_report.stats, owned_report.stats);
}

#[test]
fn count_query_matches_with_stats_uses_root_coverage_without_owned_results() {
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
        &[Vector::new(vec![0.0, 0.0]), Vector::new(vec![2.0, 2.0])],
    );

    let right_child = PartitionNode::from_points(
        2,
        &[Vector::new(vec![8.0, 8.0]), Vector::new(vec![10.0, 10.0])],
    );

    let index = FSEIndex::new(vec![root, left_child, right_child], 0);
    let query = QueryRegion::new(vec![-1.0, -1.0], vec![11.0, 11.0]);

    let report = count_query_matches_with_stats(&index, &query);

    assert_eq!(report.matched_records, 4);
    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.total_leaves, 2);
    assert_eq!(report.stats.retained_leaves, 2);
    assert_eq!(report.stats.retained_leaf_ratio, 1.0);
    assert_eq!(report.stats.total_records, 4);
    assert_eq!(report.stats.reconstructed_records, 4);
    assert_eq!(report.stats.matched_records, 4);
    assert_eq!(report.stats.candidate_ratio, 1.0);
}
