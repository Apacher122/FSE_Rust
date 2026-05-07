use crate::build::{BuildConfig, FSEBuilder, validate_index};
use crate::math::Vector;

#[test]
fn validate_index_returns_valid_report_for_builder_output() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![8.0, 8.0]),
        Vector::new(vec![9.0, 9.0]),
    ];

    let config = BuildConfig {
        max_leaf_size: 2,
        max_depth: 8,
    };
    let builder = FSEBuilder::new(config.clone());
    let index = builder.build(&points);

    let report = validate_index(&index, config.max_leaf_size);
    assert!(report.is_valid());
}
