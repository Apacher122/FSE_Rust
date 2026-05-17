use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::QueryRegion;
use crate::query::execution::{
    QueryExecutionMode, QueryExecutionOptions, execute_query_with_options,
    execute_query_with_stats_and_options, execute_retained_leaves, execute_retained_leaves_serial,
    execute_retained_leaves_with_options,
};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn query_execution_options_default_to_serial_mode() {
    let options = QueryExecutionOptions::default();

    assert_eq!(options.mode, QueryExecutionMode::Serial);
}

#[test]
fn query_execution_options_can_be_constructed_for_serial_mode() {
    let options = QueryExecutionOptions::serial();

    assert_eq!(options.mode, QueryExecutionMode::Serial);
}

#[test]
fn execute_query_with_options_matches_default_serial_query_execution() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);

    let results = execute_query_with_options(&index, &query, QueryExecutionOptions::serial());

    assert_eq!(
        results,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0]),]
    );
}

#[test]
fn execute_query_with_stats_and_options_reports_serial_execution_work() {
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

    let report =
        execute_query_with_stats_and_options(&index, &query, QueryExecutionOptions::serial());

    assert_eq!(
        report.results,
        vec![Vector::new(vec![2.0, 2.0]), Vector::new(vec![8.0, 8.0]),]
    );

    assert_eq!(report.stats.visited_nodes, 3);
    assert_eq!(report.stats.total_leaves, 2);
    assert_eq!(report.stats.retained_leaves, 2);
    assert_eq!(report.stats.retained_leaf_ratio, 1.0);
    assert_eq!(report.stats.total_records, 4);
    assert_eq!(report.stats.reconstructed_records, 4);
    assert_eq!(report.stats.matched_records, 2);
    assert_eq!(report.stats.candidate_ratio, 1.0);
}

#[test]
fn retained_leaf_options_dispatch_matches_serial_execution() {
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

    let default_report = execute_retained_leaves(&index, &query, &[1, 2]);
    let serial_report = execute_retained_leaves_serial(&index, &query, &[1, 2]);
    let options_report = execute_retained_leaves_with_options(
        &index,
        &query,
        &[1, 2],
        QueryExecutionOptions::serial(),
    );

    assert_eq!(default_report, serial_report);
    assert_eq!(serial_report, options_report);
}
