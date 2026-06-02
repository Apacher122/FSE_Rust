use crate::build::{
    BuildConfig, FSEBuilder, validate_hierarchy_topology, validate_index,
    validate_leaf_cardinality, validate_leaf_ownership_cardinality,
    validate_leaf_reconstruction_metadata, validate_leaf_record_bounds,
    validate_node_identifier_consistency, validate_parent_child_bounds,
    validate_partition_dimensional_metadata,
};
use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

#[test]
fn validate_leaf_cardinality_returns_true_when_builder_respects_limit() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
        Vector::new(vec![4.0, 4.0]),
        Vector::new(vec![5.0, 5.0]),
    ];

    let config = BuildConfig::new(2, 8);
    let builder = FSEBuilder::new(config.clone());
    let index = builder.build(&points);

    assert!(validate_leaf_cardinality(&index, config.max_leaf_size));
}

#[test]
fn validate_leaf_cardinality_returns_false_when_leaf_exceeds_limit() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);

    assert!(!validate_leaf_cardinality(&index, 2));
}

#[test]
fn validate_leaf_reconstruction_metadata_returns_true_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(validate_leaf_reconstruction_metadata(&index));
}

#[test]
fn validate_leaf_reconstruction_metadata_rejects_cached_leaf_count_mismatch() {
    let mut index = two_leaf_test_index();
    index.leaf_count = 1;

    assert!(!validate_leaf_reconstruction_metadata(&index));
}

#[test]
fn validate_leaf_reconstruction_metadata_rejects_cached_leaf_id_mismatch() {
    let mut index = two_leaf_test_index();
    index.leaf_node_ids = vec![2, 1];

    assert!(!validate_leaf_reconstruction_metadata(&index));
}

#[test]
fn validate_leaf_reconstruction_metadata_rejects_cached_shape_mismatch() {
    let mut index = two_leaf_test_index();
    index.leaf_reconstruction_shapes[0] = LeafReconstructionShape::new(1, 2, 99);

    assert!(!validate_leaf_reconstruction_metadata(&index));
}

#[test]
fn validate_leaf_reconstruction_metadata_rejects_cached_lookup_mismatch() {
    let mut index = two_leaf_test_index();
    index.leaf_reconstruction_shapes_by_node[1] = None;

    assert!(!validate_leaf_reconstruction_metadata(&index));
}

#[test]
fn validate_node_identifier_consistency_returns_true_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(validate_node_identifier_consistency(&index));
}

#[test]
fn validate_node_identifier_consistency_rejects_position_mismatch() {
    let node = PartitionNode::new(
        7,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![node], 0);

    assert!(!validate_node_identifier_consistency(&index));
}

#[test]
fn validate_partition_dimensional_metadata_returns_true_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(validate_partition_dimensional_metadata(&index));
}

#[test]
fn validate_partition_dimensional_metadata_rejects_index_dimension_mismatch() {
    let mut index = two_leaf_test_index();
    index.dimensions = 3;

    assert!(!validate_partition_dimensional_metadata(&index));
}

#[test]
fn validate_partition_dimensional_metadata_rejects_malformed_bounds_shape() {
    let mut index = two_leaf_test_index();
    index.nodes[0].bounds.max.pop();

    assert!(!validate_partition_dimensional_metadata(&index));
    assert!(!validate_parent_child_bounds(&index));
}

#[test]
fn validate_partition_dimensional_metadata_rejects_invalid_bounds_range() {
    let mut index = two_leaf_test_index();
    index.nodes[0].bounds.min[0] = 4.0;

    assert!(!validate_partition_dimensional_metadata(&index));
    assert!(!validate_parent_child_bounds(&index));
}

#[test]
fn validate_partition_dimensional_metadata_rejects_inconsistent_residual_shape() {
    let mut index = two_leaf_test_index();
    index.nodes[1].residuals.dimensions[0].push(1.0);

    assert!(!validate_partition_dimensional_metadata(&index));
}

#[test]
fn validate_partition_dimensional_metadata_rejects_leaf_cardinality_shape_mismatch() {
    let mut index = two_leaf_test_index();
    index.nodes[1].cardinality = 2;

    assert!(!validate_partition_dimensional_metadata(&index));
}

#[test]
fn validate_partition_dimensional_metadata_rejects_internal_stored_rows_above_cardinality() {
    let mut index = two_leaf_test_index();
    index.nodes[0].residuals = ResidualBlock::new(vec![vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 2.0]]);

    assert!(!validate_partition_dimensional_metadata(&index));
}

#[test]
fn validate_hierarchy_topology_returns_true_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(validate_hierarchy_topology(&index));
}

#[test]
fn validate_hierarchy_topology_rejects_leaf_with_children() {
    let node = PartitionNode::new(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        vec![0],
        true,
    );

    let index = FSEIndex::new(vec![node], 0);

    assert!(!validate_hierarchy_topology(&index));
}

#[test]
fn validate_hierarchy_topology_rejects_internal_node_without_children() {
    let node = PartitionNode::new(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        false,
    );

    let index = FSEIndex::new(vec![node], 0);

    assert!(!validate_hierarchy_topology(&index));
}

#[test]
fn validate_hierarchy_topology_rejects_unreachable_nodes() {
    let root = PartitionNode::from_points(
        0,
        &[Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])],
    );

    let unreachable = PartitionNode::from_points(
        1,
        &[Vector::new(vec![10.0, 10.0]), Vector::new(vec![11.0, 11.0])],
    );

    let index = FSEIndex::new(vec![root, unreachable], 0);

    assert!(!validate_hierarchy_topology(&index));
}

#[test]
fn validate_hierarchy_topology_rejects_direct_self_reference() {
    let node = PartitionNode::new(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        vec![0],
        false,
    );

    let index = FSEIndex::new(vec![node], 0);

    assert!(!validate_hierarchy_topology(&index));
}

#[test]
fn validate_hierarchy_topology_rejects_direct_self_reference_by_node_position() {
    let node = PartitionNode::with_cardinality(
        7,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        0,
        vec![0],
        false,
    );

    let index = FSEIndex::new(vec![node], 0);

    assert!(!validate_hierarchy_topology(&index));
}

#[test]
fn validate_parent_child_bounds_returns_true_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(validate_parent_child_bounds(&index));
}

#[test]
fn validate_parent_child_bounds_rejects_child_outside_parent_bounds() {
    let root = PartitionNode::new(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![5.0, 5.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        vec![1],
        false,
    );

    let child = PartitionNode::new(
        1,
        vec![10.0, 10.0],
        BoundingBox::new(vec![10.0, 10.0], vec![12.0, 12.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![root, child], 0);

    assert!(!validate_parent_child_bounds(&index));
}

#[test]
fn validate_parent_child_bounds_accepts_child_sharing_parent_boundary() {
    let root = PartitionNode::new(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        vec![1],
        false,
    );

    let child = PartitionNode::new(
        1,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![5.0, 5.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![root, child], 0);

    assert!(validate_parent_child_bounds(&index));
}

#[test]
fn validate_leaf_record_bounds_returns_true_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(validate_leaf_record_bounds(&index));
}

#[test]
fn validate_leaf_record_bounds_rejects_reconstructed_row_outside_leaf_bounds() {
    let node = PartitionNode::new(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![2.0], vec![0.5]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![node], 0);

    assert!(!validate_leaf_record_bounds(&index));
}

#[test]
fn validate_leaf_record_bounds_accepts_boundary_values() {
    let node = PartitionNode::new(
        0,
        vec![0.5, 0.5],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![-0.5, 0.5], vec![-0.5, 0.5]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![node], 0);

    assert!(validate_leaf_record_bounds(&index));
}

#[test]
fn validate_leaf_record_bounds_rejects_malformed_leaf_bounds_shape() {
    let mut index = two_leaf_test_index();
    index.nodes[1].bounds.max.pop();

    assert!(!validate_leaf_record_bounds(&index));
}

#[test]
fn validate_leaf_ownership_cardinality_returns_true_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(validate_leaf_ownership_cardinality(&index));
}

#[test]
fn validate_leaf_ownership_cardinality_rejects_internal_cardinality_mismatch() {
    let root = PartitionNode::with_cardinality(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        3,
        vec![1, 2],
        false,
    );

    let left = PartitionNode::new(
        1,
        vec![1.0, 1.0],
        BoundingBox::new(vec![1.0, 1.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let right = PartitionNode::new(
        2,
        vec![2.0, 2.0],
        BoundingBox::new(vec![2.0, 2.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![root, left, right], 0);

    assert!(!validate_leaf_ownership_cardinality(&index));
}

#[test]
fn validate_leaf_ownership_cardinality_rejects_shared_child_leaf() {
    let root = PartitionNode::with_cardinality(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        2,
        vec![1, 1],
        false,
    );

    let child = PartitionNode::new(
        1,
        vec![1.0, 1.0],
        BoundingBox::new(vec![1.0, 1.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![root, child], 0);

    assert!(!validate_leaf_ownership_cardinality(&index));
}

#[test]
fn validate_index_returns_valid_report_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let config = BuildConfig::new(2, 8);
    let builder = FSEBuilder::new(config.clone());
    let index = builder.build(&points);

    let report = validate_index(&index, config.max_leaf_size);
    assert!(report.node_identifier_consistency_valid);
    assert!(report.partition_dimensional_metadata_valid);
    assert!(report.leaf_cardinality_valid);
    assert!(report.leaf_reconstruction_metadata_valid);
    assert!(report.leaf_record_bounds_valid);
    assert!(report.leaf_ownership_cardinality_valid);
    assert!(report.hierarchy_topology_valid);
    assert!(report.parent_child_bounds_valid);
    assert!(report.is_valid());
}

#[test]
fn validate_index_reports_leaf_cardinality_failure() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let index = builder.build(&points);

    let report = validate_index(&index, 2);

    assert!(report.node_identifier_consistency_valid);
    assert!(report.partition_dimensional_metadata_valid);
    assert!(!report.leaf_cardinality_valid);
    assert!(report.leaf_reconstruction_metadata_valid);
    assert!(report.leaf_record_bounds_valid);
    assert!(report.leaf_ownership_cardinality_valid);
    assert!(report.hierarchy_topology_valid);
    assert!(report.parent_child_bounds_valid);
    assert!(!report.is_valid());
}

#[test]
fn validate_index_reports_leaf_record_bounds_failure() {
    let node = PartitionNode::new(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.25], vec![1.5]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![node], 0);
    let report = validate_index(&index, 8);

    assert!(report.node_identifier_consistency_valid);
    assert!(report.partition_dimensional_metadata_valid);
    assert!(report.leaf_cardinality_valid);
    assert!(report.leaf_reconstruction_metadata_valid);
    assert!(!report.leaf_record_bounds_valid);
    assert!(report.leaf_ownership_cardinality_valid);
    assert!(report.hierarchy_topology_valid);
    assert!(report.parent_child_bounds_valid);
    assert!(!report.is_valid());
}

#[test]
fn validate_index_reports_leaf_ownership_cardinality_failure() {
    let root = PartitionNode::with_cardinality(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        3,
        vec![1, 2],
        false,
    );

    let left = PartitionNode::new(
        1,
        vec![1.0, 1.0],
        BoundingBox::new(vec![1.0, 1.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let right = PartitionNode::new(
        2,
        vec![2.0, 2.0],
        BoundingBox::new(vec![2.0, 2.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![root, left, right], 0);
    let report = validate_index(&index, 8);

    assert!(report.node_identifier_consistency_valid);
    assert!(report.partition_dimensional_metadata_valid);
    assert!(report.leaf_cardinality_valid);
    assert!(report.leaf_reconstruction_metadata_valid);
    assert!(report.leaf_record_bounds_valid);
    assert!(!report.leaf_ownership_cardinality_valid);
    assert!(report.hierarchy_topology_valid);
    assert!(report.parent_child_bounds_valid);
    assert!(!report.is_valid());
}

#[test]
fn validate_index_reports_node_identifier_consistency_failure() {
    let node = PartitionNode::new(
        7,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![node], 0);
    let report = validate_index(&index, 8);

    assert!(!report.node_identifier_consistency_valid);
    assert!(report.partition_dimensional_metadata_valid);
    assert!(report.leaf_cardinality_valid);
    assert!(report.leaf_reconstruction_metadata_valid);
    assert!(report.leaf_record_bounds_valid);
    assert!(report.leaf_ownership_cardinality_valid);
    assert!(report.hierarchy_topology_valid);
    assert!(report.parent_child_bounds_valid);
    assert!(!report.is_valid());
}

#[test]
fn validate_index_reports_partition_dimensional_metadata_failure() {
    let mut index = two_leaf_test_index();
    index.nodes[0].bounds.max.pop();

    let report = validate_index(&index, 8);

    assert!(report.node_identifier_consistency_valid);
    assert!(!report.partition_dimensional_metadata_valid);
    assert!(report.leaf_cardinality_valid);
    assert!(report.leaf_reconstruction_metadata_valid);
    assert!(report.leaf_record_bounds_valid);
    assert!(report.leaf_ownership_cardinality_valid);
    assert!(report.hierarchy_topology_valid);
    assert!(!report.parent_child_bounds_valid);
    assert!(!report.is_valid());
}

#[test]
fn validate_index_reports_leaf_reconstruction_metadata_failure() {
    let mut index = two_leaf_test_index();
    index.leaf_reconstruction_shapes_by_node[1] = None;

    let report = validate_index(&index, 8);

    assert!(report.node_identifier_consistency_valid);
    assert!(report.partition_dimensional_metadata_valid);
    assert!(report.leaf_cardinality_valid);
    assert!(!report.leaf_reconstruction_metadata_valid);
    assert!(report.leaf_record_bounds_valid);
    assert!(report.leaf_ownership_cardinality_valid);
    assert!(report.hierarchy_topology_valid);
    assert!(report.parent_child_bounds_valid);
    assert!(!report.is_valid());
}

#[test]
fn validation_report_requires_all_checks_to_pass() {
    let report = crate::build::IndexValidationReport {
        node_identifier_consistency_valid: true,
        partition_dimensional_metadata_valid: true,
        leaf_cardinality_valid: true,
        leaf_reconstruction_metadata_valid: true,
        leaf_record_bounds_valid: true,
        leaf_ownership_cardinality_valid: true,
        hierarchy_topology_valid: true,
        parent_child_bounds_valid: false,
    };

    assert!(!report.is_valid());
}

fn two_leaf_test_index() -> FSEIndex {
    let root = PartitionNode::with_cardinality(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        2,
        vec![1, 2],
        false,
    );

    let left = PartitionNode::new(
        1,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![0.0, 0.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    let right = PartitionNode::new(
        2,
        vec![2.0, 2.0],
        BoundingBox::new(vec![2.0, 2.0], vec![2.0, 2.0]),
        ResidualBlock::new(vec![vec![0.0], vec![0.0]]),
        Vec::new(),
        true,
    );

    FSEIndex::new(vec![root, left, right], 0)
}
