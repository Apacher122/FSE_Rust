use crate::build::{index_density, partition_density};
use crate::math::{BoundingBox, ResidualBlock};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn bounding_box_volume_multiplies_dimension_widths() {
    let bounds = BoundingBox::new(vec![0.0, 0.0], vec![2.0, 4.0]);

    assert_eq!(bounds.volume(), 8.0);
}

#[test]
fn bounding_box_volume_is_zero_for_degenerate_dimension() {
    let bounds = BoundingBox::new(vec![1.0, 0.0], vec![1.0, 4.0]);

    assert_eq!(bounds.volume(), 0.0);
}

#[test]
fn partition_density_divides_cardinality_by_bounding_volume() {
    let node = PartitionNode::new(
        0,
        vec![1.0, 2.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 4.0]),
        ResidualBlock::new(vec![vec![0.0, 1.0], vec![0.0, 1.0]]),
        Vec::new(),
        true,
    );
    assert_eq!(partition_density(&node), 0.25);
}

#[test]
fn partition_density_is_infinite_for_non_empty_zero_volume_partition() {
    let node = PartitionNode::new(
        0,
        vec![1.0, 2.0],
        BoundingBox::new(vec![1.0, 2.0], vec![1.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    assert!(partition_density(&node).is_infinite());
}

#[test]
fn index_density_aggregates_node_cardinality_and_volume() {
    let left = PartitionNode::new(
        0,
        vec![1.0, 1.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0, 1.0], vec![0.0, 1.0]]),
        Vec::new(),
        true,
    );

    let right = PartitionNode::new(
        1,
        vec![4.0, 4.0],
        BoundingBox::new(vec![3.0, 3.0], vec![5.0, 5.0]),
        ResidualBlock::new(vec![vec![0.0, 1.0], vec![0.0, 1.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![left, right], 0);

    assert_eq!(index_density(&index), 0.5);
}

#[test]
fn index_density_ignores_internal_node_cardinality() {
    let root = PartitionNode::with_cardinality(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        4,
        vec![1, 2],
        false,
    );

    let left = PartitionNode::new(
        1,
        vec![1.0, 1.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0, 1.0], vec![0.0, 1.0]]),
        Vec::new(),
        true,
    );

    let right = PartitionNode::new(
        2,
        vec![4.0, 4.0],
        BoundingBox::new(vec![3.0, 3.0], vec![5.0, 5.0]),
        ResidualBlock::new(vec![vec![0.0, 1.0], vec![0.0, 1.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![root, left, right], 0);

    assert_eq!(index_density(&index), 0.5);
}
