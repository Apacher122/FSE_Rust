use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{QueryRegion, execute_query};

#[test]
fn builder_creates_single_leaf_when_points_fit_leaf_size() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let builder = FSEBuilder::new(BuildConfig::new(4, 8));
    let index = builder.build(&points);

    assert_eq!(index.node_count(), 1);
    assert!(index.root_node().is_leaf);
    assert_eq!(index.root_node().cardinality, 2);
}

#[test]
fn builder_creates_hierarchy_when_points_exceed_leaf_size() {
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
fn builder_exposes_configuration() {
    let config = BuildConfig::new(4, 12);
    let builder = FSEBuilder::new(config.clone());

    assert_eq!(builder.config(), &config);
}
