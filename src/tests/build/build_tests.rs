use crate::build::builder::accepts_split_quality;
use crate::build::{
    BuildConfig, FSEBuilder, split_quality_metrics_for_axis, validate_leaf_cardinality,
};
use crate::math::Vector;
use crate::query::{QueryRegion, execute_query};

#[test]
fn builder_creates_single_leaf_when_points_fit_target_leaf_size() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let builder = FSEBuilder::new(BuildConfig::new(4, 8));
    let index = builder.build(&points);

    assert_eq!(index.node_count(), 1);
    assert!(index.root_node().is_leaf);
    assert_eq!(index.root_node().cardinality, 2);
}

#[test]
fn builder_creates_hierarchy_when_points_exceed_leaf_size_and_split_improves_volume() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert_eq!(index.node_count(), 3);
    assert!(!index.root_node().is_leaf);
    assert_eq!(index.root_node().children.len(), 2);
    assert_eq!(index.root_node().cardinality, 4);
    assert_eq!(index.root_node().stored_cardinality(), 0);
    assert!(validate_leaf_cardinality(
        &index,
        builder.config().max_leaf_size
    ));
}

#[test]
fn builder_accepts_optional_split_when_zero_volume_extent_improves() {
    let points = skinny_line_points();

    let config = BuildConfig::new(16, 8).with_target_leaf_size(2);
    let builder = FSEBuilder::new(config);
    let index = builder.build(&points);

    assert!(index.node_count() > 1);
    assert!(!index.root_node().is_leaf);
    assert!(validate_leaf_cardinality(
        &index,
        builder.config().max_leaf_size
    ));
}

#[test]
fn builder_rejects_optional_zero_volume_split_when_extent_does_not_improve() {
    let points = identical_points();

    let config = BuildConfig::new(16, 8).with_target_leaf_size(2);
    let builder = FSEBuilder::new(config);
    let index = builder.build(&points);

    assert_eq!(index.node_count(), 1);
    assert!(index.root_node().is_leaf);
    assert_eq!(index.root_node().cardinality, points.len());
    assert!(validate_leaf_cardinality(
        &index,
        builder.config().max_leaf_size
    ));
}

#[test]
fn builder_rejects_optional_split_when_child_volume_does_not_improve_parent_volume() {
    let points = neutral_grid_points();

    let config = BuildConfig::new(16, 8).with_target_leaf_size(2);
    let builder = FSEBuilder::new(config);
    let index = builder.build(&points);

    assert_eq!(index.node_count(), 1);
    assert!(index.root_node().is_leaf);
    assert_eq!(index.root_node().cardinality, points.len());
    assert!(validate_leaf_cardinality(
        &index,
        builder.config().max_leaf_size
    ));
}

#[test]
fn builder_forces_split_when_points_exceed_hard_leaf_size_even_without_volume_improvement() {
    let points = neutral_grid_points();

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    assert!(index.node_count() > 1);
    assert!(!index.root_node().is_leaf);
    assert!(validate_leaf_cardinality(
        &index,
        builder.config().max_leaf_size
    ));
}

#[test]
fn builder_can_disable_positive_volume_gate_for_controlled_experiments() {
    let points = neutral_grid_points();

    let config = BuildConfig::new(16, 8)
        .with_target_leaf_size(2)
        .with_positive_split_volume_reduction_required(false);
    let builder = FSEBuilder::new(config);
    let index = builder.build(&points);

    assert!(index.node_count() > 1);
    assert!(!index.root_node().is_leaf);
    assert!(validate_leaf_cardinality(
        &index,
        builder.config().max_leaf_size
    ));
}

#[test]
fn build_config_uses_max_leaf_size_as_default_target_leaf_size() {
    let config = BuildConfig::new(4, 8);

    assert_eq!(config.target_leaf_size, 4);
    assert_eq!(config.max_leaf_size, 4);
}

#[test]
fn build_config_can_set_target_leaf_size_below_hard_leaf_size() {
    let config = BuildConfig::new(8, 8).with_target_leaf_size(3);

    assert_eq!(config.target_leaf_size, 3);
    assert_eq!(config.max_leaf_size, 8);
}

#[test]
#[should_panic(expected = "target_leaf_size must be greater than zero")]
fn build_config_rejects_zero_target_leaf_size() {
    let _ = BuildConfig::new(8, 8).with_target_leaf_size(0);
}

#[test]
#[should_panic(expected = "target_leaf_size must not exceed max_leaf_size")]
fn build_config_rejects_target_leaf_size_above_hard_leaf_size() {
    let _ = BuildConfig::new(8, 8).with_target_leaf_size(9);
}

#[test]
fn build_config_requires_positive_split_volume_reduction_by_default() {
    let config = BuildConfig::new(4, 8);

    assert!(config.require_positive_split_volume_reduction);
}

#[test]
fn build_config_can_turn_off_positive_split_volume_requirement() {
    let config = BuildConfig::new(4, 8).with_positive_split_volume_reduction_required(false);

    assert!(!config.require_positive_split_volume_reduction);
}

#[test]
fn split_quality_acceptance_requires_positive_volume_reduction_for_normal_volume() {
    let positive_points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![0.0, 10.0]),
        Vector::new(vec![10.0, 0.0]),
        Vector::new(vec![10.0, 10.0]),
    ];

    let positive_metrics = split_quality_metrics_for_axis(&positive_points, 0);
    assert!(accepts_split_quality(&positive_metrics));

    let neutral_metrics = split_quality_metrics_for_axis(&neutral_grid_points(), 0);
    assert!(!accepts_split_quality(&neutral_metrics));
}

#[test]
fn split_quality_acceptance_uses_extent_reduction_for_zero_volume_parent() {
    let metrics = split_quality_metrics_for_axis(&skinny_line_points(), 0);

    assert_eq!(metrics.parent_volume, 0.0);
    assert_eq!(metrics.combined_child_volume, 0.0);
    assert!(metrics.reduces_extent());
    assert!(accepts_split_quality(&metrics));
}

#[test]
fn split_quality_acceptance_rejects_zero_volume_parent_without_extent_reduction() {
    let metrics = split_quality_metrics_for_axis(&identical_points(), 0);

    assert_eq!(metrics.parent_volume, 0.0);
    assert_eq!(metrics.combined_child_volume, 0.0);
    assert!(!metrics.reduces_extent());
    assert!(!accepts_split_quality(&metrics));
}

#[test]
fn built_index_executes_exact_query_results() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let index = builder.build(&points);

    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);
    let matches = execute_query(&index, &query);

    assert_eq!(
        matches,
        vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0]),]
    );
}

#[test]
fn builder_respects_max_depth() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(1, 0));
    let index = builder.build(&points);

    assert_eq!(index.node_count(), 1);
    assert!(index.root_node().is_leaf);
}

#[test]
fn build_validated_returns_index_and_validation_report() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let validated = builder.build_validated(&points);

    assert_eq!(validated.index.node_count(), 3);
    assert!(validated.validation.is_valid());
}

#[test]
fn build_validated_keeps_leaf_cardinality_valid_when_hard_split_is_forced() {
    let points = neutral_grid_points();

    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let validated = builder.build_validated(&points);

    assert!(validated.index.node_count() > 1);
    assert!(validated.validation.leaf_cardinality_valid);
    assert!(validated.validation.hierarchy_topology_valid);
    assert!(validated.validation.parent_child_bounds_valid);
    assert!(validated.validation.is_valid());
}

#[test]
fn build_validated_allows_optional_unsplit_leaf_under_hard_limit() {
    let points = neutral_grid_points();

    let config = BuildConfig::new(16, 8).with_target_leaf_size(2);
    let builder = FSEBuilder::new(config);
    let validated = builder.build_validated(&points);

    assert_eq!(validated.index.node_count(), 1);
    assert!(validated.index.root_node().is_leaf);
    assert_eq!(validated.index.root_node().cardinality, points.len());
    assert!(validated.validation.is_valid());
}

#[test]
fn build_validated_splits_zero_volume_partition_when_extent_improves() {
    let points = skinny_line_points();

    let config = BuildConfig::new(16, 8).with_target_leaf_size(2);
    let builder = FSEBuilder::new(config);
    let validated = builder.build_validated(&points);

    assert!(validated.index.node_count() > 1);
    assert!(validated.validation.is_valid());
}

#[test]
fn builder_exposes_configuration() {
    let config = BuildConfig::new(4, 12);
    let builder = FSEBuilder::new(config.clone());

    assert_eq!(builder.config(), &config);
}

fn skinny_line_points() -> Vec<Vector> {
    vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 0.0]),
        Vector::new(vec![8.0, 0.0]),
        Vector::new(vec![9.0, 0.0]),
    ]
}

fn identical_points() -> Vec<Vector> {
    vec![
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![1.0, 1.0]),
    ]
}

fn neutral_grid_points() -> Vec<Vector> {
    vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![5.0, 0.0]),
        Vector::new(vec![10.0, 0.0]),
        Vector::new(vec![0.0, 5.0]),
        Vector::new(vec![5.0, 5.0]),
        Vector::new(vec![10.0, 5.0]),
        Vector::new(vec![0.0, 10.0]),
        Vector::new(vec![5.0, 10.0]),
        Vector::new(vec![10.0, 10.0]),
    ]
}
