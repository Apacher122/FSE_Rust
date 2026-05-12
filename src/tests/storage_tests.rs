use crate::math::Vector;
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn partition_node_can_be_built_from_points() {
    let points = vec![Vector::new(vec![0.0, 2.0]), Vector::new(vec![2.0, 4.0])];
    let node = PartitionNode::from_points(0, &points);

    assert_eq!(node.id, 0);
    assert_eq!(node.dimensions(), 2);
    assert_eq!(node.cardinality, 2);
    assert_eq!(node.centroid, vec![1.0, 3.0]);
    assert_eq!(node.bounds.min, vec![0.0, 2.0]);
    assert_eq!(node.bounds.max, vec![2.0, 4.0]);
    assert!(node.is_leaf);
    assert!(!node.has_children());
}

#[test]
fn single_leaf_index_can_be_created_from_partition() {
    let points = vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![3.0, 5.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    assert_eq!(index.root, 0);
    assert_eq!(index.dimensions, 2);
    assert_eq!(index.node_count(), 1);
    assert!(index.is_single_leaf());
}

#[test]
fn root_node_returns_index_root_partition() {
    let points = vec![Vector::new(vec![1.0, 2.0]), Vector::new(vec![5.0, 6.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    assert_eq!(index.root_node().id, 0);
    assert_eq!(index.root_node().cardinality, 2);
}

#[test]
fn internal_partition_tracks_subtree_cardinality_without_stored_residuals() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let node = PartitionNode::internal_from_points(0, &points, vec![1, 2]);

    assert_eq!(node.cardinality, 3);
    assert_eq!(node.stored_cardinality(), 0);
    assert!(!node.is_leaf);
    assert_eq!(node.children, vec![1, 2]);
}
