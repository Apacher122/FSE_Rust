use crate::build::partition_density;
use crate::math::{BoundingBox, ResidualBlock};
use crate::storage::PartitionNode;

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
