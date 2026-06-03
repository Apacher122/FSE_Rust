use crate::math::{BoundingBox, CentroidError, ResidualBlock, Vector};
use crate::storage::{FSEIndex, FSEIndexError, PartitionFromPointsError, PartitionNode};

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
fn checked_partition_node_can_be_built_from_points() {
    let points = vec![Vector::new(vec![0.0, 2.0]), Vector::new(vec![2.0, 4.0])];
    let node = PartitionNode::try_from_points(0, &points)
        .expect("valid points should build a leaf partition");

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
fn checked_partition_node_reports_empty_point_set() {
    let error =
        PartitionNode::try_from_points(0, &[]).expect_err("empty point set should be rejected");

    assert_eq!(error, PartitionFromPointsError::EmptyPointSet);
    assert_eq!(
        error.to_string(),
        "cannot build a partition from an empty point set"
    );
}

#[test]
fn checked_partition_node_reports_dimension_mismatch() {
    let points = vec![Vector::new(vec![0.0, 2.0]), Vector::new(vec![4.0])];

    let error = PartitionNode::try_from_points(0, &points)
        .expect_err("mismatched point dimensions should be rejected");

    assert_eq!(
        error,
        PartitionFromPointsError::Centroid(CentroidError::DimensionMismatch {
            point: 1,
            actual_dimensions: 1,
            expected_dimensions: 2,
        })
    );
    assert_eq!(
        error.to_string(),
        "all points must have the same dimensionality"
    );
}

#[test]
fn checked_partition_node_reports_non_finite_coordinate() {
    let points = vec![Vector {
        values: vec![f32::NAN],
    }];

    let error = PartitionNode::try_from_points(0, &points)
        .expect_err("non-finite point coordinates should be rejected");

    assert_eq!(
        error,
        PartitionFromPointsError::Centroid(CentroidError::NonFiniteCoordinate {
            point: 0,
            dimension: 0,
        })
    );
    assert_eq!(error.to_string(), "point coordinates must be finite");
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
fn checked_single_leaf_index_can_be_created_from_partition() {
    let points = vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![3.0, 5.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::try_from_root(root).expect("valid root should build an index");

    assert_eq!(index.root, 0);
    assert_eq!(index.dimensions, 2);
    assert_eq!(index.node_count(), 1);
    assert!(index.is_single_leaf());
}

#[test]
fn checked_index_constructor_accepts_valid_nodes() {
    let points = vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![3.0, 5.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::try_new(vec![root], 0).expect("valid nodes should build an index");

    assert_eq!(index.root, 0);
    assert_eq!(index.dimensions, 2);
    assert_eq!(index.node_count(), 1);
    assert_eq!(index.leaf_count(), 1);
}

#[test]
fn checked_index_constructor_reports_empty_node_list() {
    let error = FSEIndex::try_new(Vec::new(), 0).expect_err("empty node list should be rejected");

    assert_eq!(error, FSEIndexError::EmptyNodeList);
    assert_eq!(error.to_string(), "index must contain at least one node");
}

#[test]
fn checked_index_constructor_reports_missing_root() {
    let points = vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![3.0, 5.0])];
    let node = PartitionNode::from_points(0, &points);

    let error = FSEIndex::try_new(vec![node], 1).expect_err("missing root should be rejected");

    assert_eq!(
        error,
        FSEIndexError::MissingRoot {
            root: 1,
            node_count: 1,
        }
    );
    assert_eq!(
        error.to_string(),
        "root node id must exist in the node list"
    );
}

#[test]
fn checked_index_constructor_reports_node_dimension_mismatch() {
    let one_dimension = PartitionNode::from_points(0, &[Vector::new(vec![1.0])]);
    let two_dimensions = PartitionNode::from_points(1, &[Vector::new(vec![1.0, 2.0])]);

    let error = FSEIndex::try_new(vec![one_dimension, two_dimensions], 0)
        .expect_err("mismatched index node dimensions should be rejected");

    assert_eq!(
        error,
        FSEIndexError::DimensionMismatch {
            node: 1,
            actual_dimensions: 2,
            expected_dimensions: 1,
        }
    );
    assert_eq!(
        error.to_string(),
        "all nodes in an index must have the same dimensionality"
    );
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

#[test]
fn checked_internal_partition_can_be_built_from_points() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let node = PartitionNode::try_internal_from_points(0, &points, vec![1, 2])
        .expect("valid points should build an internal partition");

    assert_eq!(node.cardinality, 3);
    assert_eq!(node.stored_cardinality(), 0);
    assert!(!node.is_leaf);
    assert_eq!(node.children, vec![1, 2]);
}

#[test]
fn checked_internal_partition_reports_empty_point_set() {
    let error = PartitionNode::try_internal_from_points(0, &[], vec![1, 2])
        .expect_err("empty point set should be rejected");

    assert_eq!(error, PartitionFromPointsError::EmptyInternalPointSet);
    assert_eq!(
        error.to_string(),
        "cannot build an internal partition from an empty point set"
    );
}

#[test]
fn internal_partition_can_store_fewer_residual_rows_than_subtree_cardinality() {
    let node = PartitionNode::with_cardinality(
        0,
        vec![1.0, 1.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        4,
        vec![1, 2],
        false,
    );

    assert_eq!(node.cardinality, 4);
    assert_eq!(node.stored_cardinality(), 1);
    assert!(!node.is_leaf);
}

#[test]
#[should_panic(expected = "leaf partition cardinality must match stored residual row count")]
fn leaf_partition_rejects_cardinality_that_does_not_match_stored_rows() {
    let _node = PartitionNode::with_cardinality(
        0,
        vec![1.0, 1.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        2,
        Vec::new(),
        true,
    );
}

#[test]
#[should_panic(expected = "stored residual rows must not exceed partition cardinality")]
fn internal_partition_rejects_more_stored_rows_than_declared_cardinality() {
    let _node = PartitionNode::with_cardinality(
        0,
        vec![1.0, 1.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0, 1.0], vec![0.0, 1.0]]),
        1,
        vec![1, 2],
        false,
    );
}

#[test]
#[should_panic(expected = "partition centroid values must be finite")]
fn partition_node_rejects_nan_centroid_value() {
    let _node = PartitionNode::with_cardinality(
        0,
        vec![f32::NAN],
        BoundingBox::new(vec![0.0], vec![1.0]),
        ResidualBlock::new(vec![vec![0.0]]),
        1,
        Vec::new(),
        true,
    );
}

#[test]
#[should_panic(expected = "partition centroid values must be finite")]
fn partition_node_rejects_infinite_centroid_value() {
    let _node = PartitionNode::with_cardinality(
        0,
        vec![f32::INFINITY],
        BoundingBox::new(vec![0.0], vec![1.0]),
        ResidualBlock::new(vec![vec![0.0]]),
        1,
        Vec::new(),
        true,
    );
}
