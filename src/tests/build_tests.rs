use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;

#[test]
fn builder_creates_hierarchy_when_points_exceed_leaf_size() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let builder = FSEBuilder::new(BuildConfig {
        max_leaf_size: 2,
        max_depth: 8,
    });
    let index = builder.build(&points);

    assert_eq!(index.node_count(), 3);
    assert!(!index.root_node().is_leaf);
}
