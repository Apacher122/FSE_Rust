use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::{
    QueryRegion, evaluate_query, execute_query, execute_query_with_stats, reconstruct_partition,
    reconstruct_row_into, traverse,
};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn query_region_contains_points_inside_its_bounds() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![2.0, 2.0]);
    let inside = Vector::new(vec![1.0, 1.5]);
    let outside = Vector::new(vec![3.0, 1.5]);

    assert!(query.contains_point(&inside));
    assert!(!query.contains_point(&outside));
}

#[test]
fn query_region_contains_raw_coordinate_values_inside_its_bounds() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![2.0, 2.0]);

    assert!(query.contains_values(&[1.0, 1.5]));
    assert!(!query.contains_values(&[3.0, 1.5]));
}

#[test]
fn traversal_retains_single_leaf_when_query_intersects_bounds() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![2.0, 2.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![1.0, 1.0], vec![3.0, 3.0]);
    let retained = traverse(&index, &query);

    assert_eq!(retained, vec![0]);
}

#[test]
fn traversal_prunes_single_leaf_when_query_is_disjoint() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![2.0, 2.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![3.0, 3.0], vec![4.0, 4.0]);
    let retained = traverse(&index, &query);

    assert!(retained.is_empty());
}

#[test]
fn traversal_descends_into_intersecting_children_only() {
    let root = PartitionNode::new(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
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

    let query = QueryRegion::new(vec![1.0, 1.0], vec![3.0, 3.0]);
    let retained = traverse(&index, &query);

    assert_eq!(retained, vec![1]);
}

#[test]
fn reconstruction_restores_original_points_from_partition_residuals() {
    let points = vec![
        Vector::new(vec![2.0, 4.0]),
        Vector::new(vec![4.0, 8.0]),
        Vector::new(vec![6.0, 12.0]),
    ];

    let node = PartitionNode::from_points(0, &points);
    let reconstructed = reconstruct_partition(&node);

    assert_eq!(reconstructed, points);
}

#[test]
fn reconstruction_can_write_single_row_into_reusable_buffer() {
    let points = vec![
        Vector::new(vec![2.0, 4.0]),
        Vector::new(vec![4.0, 8.0]),
        Vector::new(vec![6.0, 12.0]),
    ];

    let node = PartitionNode::from_points(0, &points);
    let mut values = vec![999.0, 999.0];

    reconstruct_row_into(&node, 1, &mut values);
    assert_eq!(values, vec![4.0, 8.0]);

    reconstruct_row_into(&node, 2, &mut values);
    assert_eq!(values, vec![6.0, 12.0]);
}

#[test]
fn evaluator_returns_only_points_inside_query_region() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let matches = evaluate_query(&points, &query);

    assert_eq!(
        matches,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0]),]
    );
}

#[test]
fn evaluator_treats_query_boundaries_as_inclusive() {
    let points = vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0])];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let matches = evaluate_query(&points, &query);

    assert_eq!(matches, points);
}

#[test]
fn evaluator_returns_empty_result_when_no_points_match() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![5.0, 5.0])];

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let matches = evaluate_query(&points, &query);

    assert!(matches.is_empty());
}

#[test]
fn execute_query_returns_exact_matches_from_single_leaf_index() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let matches = execute_query(&index, &query);

    assert_eq!(
        matches,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0]),]
    );
}

#[test]
fn execute_query_returns_empty_result_when_root_is_pruned() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![5.0, 5.0], vec![6.0, 6.0]);
    let matches = execute_query(&index, &query);

    assert!(matches.is_empty());
}

#[test]
fn execute_query_filters_false_positive_partition_retention() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![10.0, 10.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![4.0, 4.0], vec![6.0, 6.0]);
    let matches = execute_query(&index, &query);

    assert!(matches.is_empty());
}

#[test]
fn execute_query_returns_matches_from_retained_child_only() {
    let root = PartitionNode::new(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
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

    let query = QueryRegion::new(vec![1.0, 1.0], vec![3.0, 3.0]);
    let matches = execute_query(&index, &query);

    assert_eq!(matches, vec![Vector::new(vec![2.0, 2.0])]);
}

#[test]
fn execute_query_with_stats_reports_single_leaf_work() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let report = execute_query_with_stats(&index, &query);

    assert_eq!(
        report.results,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0]),]
    );

    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.total_leaves, 1);
    assert_eq!(report.stats.retained_leaves, 1);
    assert_eq!(report.stats.retained_leaf_ratio, 1.0);
    assert_eq!(report.stats.total_records, 3);
    assert_eq!(report.stats.reconstructed_records, 3);
    assert_eq!(report.stats.matched_records, 2);
    assert_eq!(report.stats.candidate_ratio, 1.0);
}

#[test]
fn execute_query_with_stats_reports_pruned_root_without_reconstruction() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);
    let report = execute_query_with_stats(&index, &query);

    assert!(report.results.is_empty());

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
fn execute_query_with_stats_reports_hierarchy_traversal_work() {
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

    let query = QueryRegion::new(vec![1.0, 1.0], vec![3.0, 3.0]);
    let report = execute_query_with_stats(&index, &query);

    assert_eq!(report.results, vec![Vector::new(vec![2.0, 2.0])]);

    assert_eq!(report.stats.visited_nodes, 3);
    assert_eq!(report.stats.total_leaves, 2);
    assert_eq!(report.stats.retained_leaves, 1);
    assert_eq!(report.stats.retained_leaf_ratio, 0.5);
    assert_eq!(report.stats.total_records, 4);
    assert_eq!(report.stats.reconstructed_records, 2);
    assert_eq!(report.stats.matched_records, 1);
    assert_eq!(report.stats.candidate_ratio, 0.5);
}
