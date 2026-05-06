use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::{QueryRegion, evaluate_query, reconstruct_partition, traverse};
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
