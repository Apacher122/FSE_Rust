use crate::benchmark::workloads::large_clustered_points_2d;
use crate::build::{BuildConfig, FSEBuilder, index_validation_diagnostics};
use crate::math::{BoundingBox, ResidualBlock};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn validation_diagnostics_reports_large_leaf_cardinality_violations() {
    let points = large_clustered_points_2d();
    let config = BuildConfig::new(8, 8).with_target_leaf_size(8);
    let builder = FSEBuilder::new(config);
    let validated = builder.build_validated(&points);

    assert!(!validated.validation.is_valid());
    assert!(!validated.validation.leaf_cardinality_valid);

    let diagnostics = index_validation_diagnostics(&validated.index, 8);

    assert!(!diagnostics.leaf_cardinality_violations.is_empty());

    let worst_leaf = diagnostics
        .leaf_cardinality_violations
        .iter()
        .max_by_key(|violation| violation.cardinality)
        .expect("large validation failure should report a worst leaf");

    assert!(
        worst_leaf.cardinality > worst_leaf.max_leaf_size,
        "worst leaf should exceed the configured max leaf size"
    );
    assert_eq!(
        worst_leaf.overflow_by,
        worst_leaf.cardinality - worst_leaf.max_leaf_size
    );
}

#[test]
fn validation_diagnostics_reports_no_large_topology_or_bounds_violations() {
    let points = large_clustered_points_2d();
    let config = BuildConfig::new(8, 8).with_target_leaf_size(8);
    let builder = FSEBuilder::new(config);
    let validated = builder.build_validated(&points);
    let diagnostics = index_validation_diagnostics(&validated.index, 8);

    assert!(validated.validation.hierarchy_topology_valid);
    assert!(validated.validation.leaf_record_bounds_valid);
    assert!(validated.validation.parent_child_bounds_valid);
    assert_eq!(diagnostics.leaf_record_bounds_violations.len(), 0);
    assert!(diagnostics.hierarchy_topology.root_valid);
    assert_eq!(
        diagnostics
            .hierarchy_topology
            .invalid_child_references
            .len(),
        0
    );
    assert_eq!(diagnostics.hierarchy_topology.self_reference_count, 0);
    assert_eq!(
        diagnostics
            .hierarchy_topology
            .leaf_nodes_with_children_count,
        0
    );
    assert_eq!(
        diagnostics
            .hierarchy_topology
            .internal_nodes_without_children_count,
        0
    );
    assert_eq!(diagnostics.hierarchy_topology.unreachable_node_count, 0);
    assert_eq!(diagnostics.parent_child_bounds_violations.len(), 0);
}

#[test]
fn validation_diagnostics_reports_leaf_record_bounds_violations() {
    let node = PartitionNode::new(
        0,
        vec![0.0, 0.0],
        BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]),
        ResidualBlock::new(vec![vec![2.0], vec![0.5]]),
        Vec::new(),
        true,
    );

    let index = FSEIndex::new(vec![node], 0);
    let diagnostics = index_validation_diagnostics(&index, 8);

    assert_eq!(diagnostics.leaf_record_bounds_violations.len(), 1);

    let violation = &diagnostics.leaf_record_bounds_violations[0];

    assert_eq!(violation.node_id, 0);
    assert_eq!(violation.row, 0);
    assert_eq!(violation.dimension, 0);
    assert_eq!(violation.value, 2.0);
    assert_eq!(violation.minimum, 0.0);
    assert_eq!(violation.maximum, 1.0);
}
