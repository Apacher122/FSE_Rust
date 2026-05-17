use crate::build::{
    IndexStructureMetrics, SplitQualityMetrics, bounding_extent_sum, index_density,
    index_structure_metrics, partition_density, split_quality_metrics,
    split_quality_metrics_for_axis, split_quality_metrics_from_bounds,
};
use crate::math::{BoundingBox, ResidualBlock, Vector};
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
    let index = two_leaf_index();

    assert_eq!(index_density(&index), 0.5);
}

#[test]
fn index_density_ignores_internal_node_cardinality() {
    let index = internal_plus_two_leaf_index();

    assert_eq!(index_density(&index), 0.5);
}

#[test]
fn index_structure_metrics_summarize_leaf_layout() {
    let index = internal_plus_two_leaf_index();

    let metrics = index_structure_metrics(&index);

    assert_eq!(
        metrics,
        IndexStructureMetrics {
            node_count: 3,
            leaf_count: 2,
            internal_node_count: 1,
            total_leaf_cardinality: 4,
            min_leaf_cardinality: 2,
            max_leaf_cardinality: 2,
            average_leaf_cardinality: 2.0,
            total_leaf_volume: 8.0,
            average_leaf_volume: 4.0,
            index_density: 0.5,
            zero_volume_leaf_count: 0,
        }
    );
    assert!(!metrics.is_empty());
}

#[test]
fn index_structure_metrics_counts_zero_volume_leaves() {
    let left = PartitionNode::new(
        0,
        vec![1.0, 1.0],
        BoundingBox::new(vec![1.0, 1.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
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
    let metrics = index_structure_metrics(&index);

    assert_eq!(metrics.leaf_count, 2);
    assert_eq!(metrics.zero_volume_leaf_count, 1);
    assert_eq!(metrics.total_leaf_cardinality, 3);
    assert_eq!(metrics.total_leaf_volume, 4.0);
    assert_eq!(metrics.index_density, 0.75);
}

#[test]
fn index_structure_metrics_handles_single_leaf_index() {
    let leaf = PartitionNode::new(
        0,
        vec![1.0, 1.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0, 1.0], vec![0.0, 1.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![leaf], 0);
    let metrics = index_structure_metrics(&index);

    assert_eq!(metrics.node_count, 1);
    assert_eq!(metrics.leaf_count, 1);
    assert_eq!(metrics.internal_node_count, 0);
    assert_eq!(metrics.total_leaf_cardinality, 2);
    assert_eq!(metrics.min_leaf_cardinality, 2);
    assert_eq!(metrics.max_leaf_cardinality, 2);
    assert_eq!(metrics.average_leaf_cardinality, 2.0);
    assert_eq!(metrics.total_leaf_volume, 4.0);
    assert_eq!(metrics.average_leaf_volume, 4.0);
    assert_eq!(metrics.index_density, 0.5);
    assert_eq!(metrics.zero_volume_leaf_count, 0);
    assert!(!metrics.is_empty());
}
#[test]
fn bounding_extent_sum_adds_dimension_widths() {
    let bounds = BoundingBox::new(vec![0.0, 1.0, 2.0], vec![2.0, 5.0, 8.0]);

    assert_eq!(bounding_extent_sum(&bounds), 12.0);
}

#[test]
fn split_quality_metrics_reports_volume_and_extent_reduction() {
    let parent = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![0.0, 2.0]),
        Vector::new(vec![2.0, 0.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let left = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![0.0, 2.0])];

    let right = vec![Vector::new(vec![2.0, 0.0]), Vector::new(vec![2.0, 2.0])];

    let metrics = split_quality_metrics(&parent, &left, &right);

    assert_eq!(metrics.parent_volume, 4.0);
    assert_eq!(metrics.combined_child_volume, 0.0);
    assert_eq!(metrics.volume_reduction_ratio, 1.0);
    assert_eq!(metrics.parent_extent, 4.0);
    assert_eq!(metrics.combined_child_extent, 4.0);
    assert_eq!(metrics.extent_reduction_ratio, 0.0);
    assert_eq!(metrics.parent_cardinality, 4);
    assert_eq!(metrics.left_cardinality, 2);
    assert_eq!(metrics.right_cardinality, 2);
    assert_eq!(metrics.balance_penalty, 0);
    assert!(metrics.reduces_volume());
    assert!(!metrics.reduces_extent());
    assert!(metrics.is_balanced());
}

#[test]
fn split_quality_metrics_can_report_negative_volume_reduction_for_overlapping_child_boxes() {
    let parent = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![10.0, 10.0]),
        Vector::new(vec![0.0, 10.0]),
        Vector::new(vec![10.0, 0.0]),
    ];

    let left = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![10.0, 10.0])];

    let right = vec![Vector::new(vec![0.0, 10.0]), Vector::new(vec![10.0, 0.0])];

    let metrics = split_quality_metrics(&parent, &left, &right);

    // ugly split child boxes overlap the whole parent
    assert_eq!(metrics.parent_volume, 100.0);
    assert_eq!(metrics.combined_child_volume, 200.0);
    assert_eq!(metrics.volume_reduction_ratio, -1.0);
    assert!(!metrics.reduces_volume());
}

#[test]
fn split_quality_metrics_for_axis_measures_median_split_geometry() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![0.0, 1.0]),
        Vector::new(vec![1.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
    ];

    let metrics = split_quality_metrics_for_axis(&points, 0);

    assert_eq!(metrics.parent_volume, 1.0);
    assert_eq!(metrics.combined_child_volume, 0.0);
    assert_eq!(metrics.volume_reduction_ratio, 1.0);
    assert_eq!(metrics.parent_cardinality, 4);
    assert_eq!(metrics.left_cardinality, 2);
    assert_eq!(metrics.right_cardinality, 2);
}

#[test]
fn split_quality_metrics_for_axis_distinguishes_volume_sensitive_axes() {
    let points = volume_sensitive_points();

    let axis_zero_metrics = split_quality_metrics_for_axis(&points, 0);
    let axis_one_metrics = split_quality_metrics_for_axis(&points, 1);

    assert_eq!(axis_zero_metrics.combined_child_volume, 9.0);
    assert_eq!(axis_one_metrics.combined_child_volume, 1.0);
    assert!(axis_one_metrics.volume_reduction_ratio > axis_zero_metrics.volume_reduction_ratio);
}

#[test]
fn split_quality_metrics_from_bounds_uses_supplied_bounds() {
    let parent_bounds = BoundingBox::new(vec![0.0, 0.0], vec![4.0, 4.0]);
    let left_bounds = BoundingBox::new(vec![0.0, 0.0], vec![2.0, 4.0]);
    let right_bounds = BoundingBox::new(vec![2.0, 0.0], vec![4.0, 4.0]);

    let metrics =
        split_quality_metrics_from_bounds(&parent_bounds, &left_bounds, &right_bounds, 8, 4, 4);

    assert_eq!(
        metrics,
        SplitQualityMetrics {
            parent_volume: 16.0,
            combined_child_volume: 16.0,
            volume_reduction_ratio: 0.0,
            parent_extent: 8.0,
            combined_child_extent: 12.0,
            extent_reduction_ratio: -0.5,
            parent_cardinality: 8,
            left_cardinality: 4,
            right_cardinality: 4,
            balance_penalty: 0,
        }
    );
}

#[test]
#[should_panic(expected = "parent point set must not be empty")]
fn split_quality_metrics_rejects_empty_parent_points() {
    let left = vec![Vector::new(vec![0.0, 0.0])];
    let right = vec![Vector::new(vec![1.0, 1.0])];

    let _ = split_quality_metrics(&[], &left, &right);
}

#[test]
#[should_panic(expected = "child point counts must add up to parent point count")]
fn split_quality_metrics_rejects_cardinality_mismatch() {
    let parent = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let left = vec![Vector::new(vec![0.0, 0.0])];
    let right = vec![Vector::new(vec![1.0, 1.0])];

    let _ = split_quality_metrics(&parent, &left, &right);
}

fn two_leaf_index() -> FSEIndex {
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

    FSEIndex::new(vec![left, right], 0)
}

fn internal_plus_two_leaf_index() -> FSEIndex {
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

    FSEIndex::new(vec![root, left, right], 0)
}

fn volume_sensitive_points() -> Vec<Vector> {
    vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![0.0, 1.0]),
        Vector::new(vec![0.0, 2.0]),
        Vector::new(vec![1.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![10.0, 0.0]),
    ]
}
