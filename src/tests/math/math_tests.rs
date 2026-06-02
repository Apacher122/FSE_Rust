use crate::math::{
    BoundingBox, BoundingBoxError, ResidualBlock, ResidualBlockError, Vector, VectorError,
};

#[test]
fn coordinate_vector_accepts_finite_coordinates() {
    let vector = Vector::new(vec![1.0, 2.0]);

    assert_eq!(vector.values, vec![1.0, 2.0]);
    assert_eq!(vector.dimensions(), 2);
    assert!(!vector.is_empty());
}

#[test]
fn coordinate_vector_checked_constructor_accepts_finite_coordinates() {
    let vector =
        Vector::try_new(vec![1.0, 2.0]).expect("valid coordinate vector should be accepted");

    assert_eq!(vector.values, vec![1.0, 2.0]);
    assert_eq!(vector.dimensions(), 2);
    assert!(!vector.is_empty());
}

#[test]
fn coordinate_vector_checked_constructor_reports_empty_coordinates() {
    let error =
        Vector::try_new(Vec::new()).expect_err("empty coordinate vector should be rejected");

    assert_eq!(error, VectorError::Empty);
    assert_eq!(
        error.to_string(),
        "coordinate vector must have at least one dimension"
    );
}

#[test]
fn coordinate_vector_checked_constructor_reports_nan_coordinate() {
    let error = Vector::try_new(vec![1.0, f32::NAN])
        .expect_err("non-finite coordinate vector should be rejected");

    assert_eq!(error, VectorError::NonFinite { dimension: 1 });
    assert_eq!(error.to_string(), "coordinate vector values must be finite");
}

#[test]
fn coordinate_vector_checked_constructor_reports_infinite_coordinate() {
    let error = Vector::try_new(vec![f32::INFINITY])
        .expect_err("non-finite coordinate vector should be rejected");

    assert_eq!(error, VectorError::NonFinite { dimension: 0 });
    assert_eq!(error.to_string(), "coordinate vector values must be finite");
}

#[test]
#[should_panic(expected = "coordinate vector must have at least one dimension")]
fn coordinate_vector_rejects_empty_coordinates() {
    let _ = Vector::new(Vec::new());
}

#[test]
#[should_panic(expected = "coordinate vector values must be finite")]
fn coordinate_vector_rejects_nan_coordinate() {
    let _ = Vector::new(vec![f32::NAN]);
}

#[test]
#[should_panic(expected = "coordinate vector values must be finite")]
fn coordinate_vector_rejects_infinite_coordinate() {
    let _ = Vector::new(vec![f32::INFINITY]);
}

#[test]
fn bounding_box_contains_points_used_to_build_it() {
    let points = vec![
        Vector::new(vec![1.0, 2.0]),
        Vector::new(vec![3.0, 4.0]),
        Vector::new(vec![2.0, 1.0]),
    ];

    let bounds = BoundingBox::from_points(&points);

    for point in &points {
        assert!(bounds.contains_point(point));
    }

    assert_eq!(bounds.min, vec![1.0, 1.0]);
    assert_eq!(bounds.max, vec![3.0, 4.0]);
}

#[test]
fn bounding_boxes_intersect_when_ranges_overlap() {
    let left = BoundingBox::new(vec![0.0, 0.0], vec![2.0, 2.0]);
    let right = BoundingBox::new(vec![1.0, 1.0], vec![3.0, 3.0]);

    assert!(left.intersects(&right));
}

#[test]
fn bounding_boxes_do_not_intersect_when_any_dimension_is_disjoint() {
    let left = BoundingBox::new(vec![0.0, 0.0], vec![1.0, 1.0]);
    let right = BoundingBox::new(vec![2.0, 0.5], vec![3.0, 0.75]);

    assert!(!left.intersects(&right));
}

#[test]
fn bounding_box_checked_constructor_accepts_valid_bounds() {
    let bounds = BoundingBox::try_new(vec![0.0, 1.0], vec![2.0, 3.0])
        .expect("valid bounding box should be accepted");

    assert_eq!(bounds.min, vec![0.0, 1.0]);
    assert_eq!(bounds.max, vec![2.0, 3.0]);
}

#[test]
fn bounding_box_checked_constructor_reports_mismatched_bound_dimensions() {
    let error = BoundingBox::try_new(vec![0.0, 1.0], vec![2.0])
        .expect_err("mismatched bounding box dimensions should be rejected");

    assert_eq!(
        error,
        BoundingBoxError::DimensionMismatch {
            min_dimensions: 2,
            max_dimensions: 1,
        }
    );
    assert_eq!(
        error.to_string(),
        "bounding box min and max vectors must have the same dimensionality"
    );
}

#[test]
fn bounding_box_checked_constructor_reports_empty_bounds() {
    let error = BoundingBox::try_new(Vec::new(), Vec::new())
        .expect_err("empty bounding box dimensions should be rejected");

    assert_eq!(error, BoundingBoxError::Empty);
    assert_eq!(
        error.to_string(),
        "bounding box must have at least one dimension"
    );
}

#[test]
fn bounding_box_checked_constructor_reports_non_finite_bounds() {
    let error = BoundingBox::try_new(vec![0.0, f32::NAN], vec![1.0, 2.0])
        .expect_err("non-finite bounding box bounds should be rejected");

    assert_eq!(error, BoundingBoxError::NonFinite { dimension: 1 });
    assert_eq!(
        error.to_string(),
        "bounding box bounds must be finite in every dimension"
    );
}

#[test]
fn bounding_box_checked_constructor_reports_inverted_bounds() {
    let error = BoundingBox::try_new(vec![0.0, 4.0], vec![1.0, 2.0])
        .expect_err("inverted bounding box bounds should be rejected");

    assert_eq!(error, BoundingBoxError::InvertedRange { dimension: 1 });
    assert_eq!(
        error.to_string(),
        "bounding box minimum must not exceed maximum in dimension 1"
    );
}

#[test]
#[should_panic(expected = "bounding box min and max vectors must have the same dimensionality")]
fn bounding_box_rejects_mismatched_bound_dimensions() {
    let _ = BoundingBox::new(vec![0.0, 1.0], vec![2.0]);
}

#[test]
#[should_panic(expected = "bounding box must have at least one dimension")]
fn bounding_box_rejects_empty_bounds() {
    let _ = BoundingBox::new(Vec::new(), Vec::new());
}

#[test]
#[should_panic(expected = "bounding box minimum must not exceed maximum in dimension 0")]
fn bounding_box_rejects_inverted_bounds() {
    let _ = BoundingBox::new(vec![3.0, 0.0], vec![2.0, 1.0]);
}

#[test]
#[should_panic(expected = "bounding box bounds must be finite in every dimension")]
fn bounding_box_rejects_nan_minimum() {
    let _ = BoundingBox::new(vec![f32::NAN], vec![1.0]);
}

#[test]
#[should_panic(expected = "bounding box bounds must be finite in every dimension")]
fn bounding_box_rejects_infinite_maximum() {
    let _ = BoundingBox::new(vec![0.0], vec![f32::INFINITY]);
}

#[test]
#[should_panic(expected = "bounding box point coordinates must be finite in every dimension")]
fn bounding_box_from_points_rejects_non_finite_coordinates() {
    let points = vec![
        Vector::new(vec![0.0]),
        Vector {
            values: vec![f32::NAN],
        },
    ];

    let _ = BoundingBox::from_points(&points);
}

#[test]
fn residual_block_encodes_points_relative_to_centroid() {
    let points = vec![Vector::new(vec![2.0, 4.0]), Vector::new(vec![4.0, 8.0])];

    let centroid = vec![3.0, 6.0];
    let residuals = ResidualBlock::from_points(&points, &centroid);

    assert_eq!(residuals.dimensions(), 2);
    assert_eq!(residuals.cardinality(), 2);
    assert_eq!(residuals.dimensions[0], vec![-1.0, 1.0]);
    assert_eq!(residuals.dimensions[1], vec![-2.0, 2.0]);
}

#[test]
fn residual_block_accepts_dimensions_with_matching_row_counts() {
    let residuals = ResidualBlock::new(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);

    assert_eq!(residuals.dimensions(), 2);
    assert_eq!(residuals.cardinality(), 2);
    assert!(residuals.has_consistent_shape());
    assert_eq!(residuals.dimension_lengths(), vec![2, 2]);
}

#[test]
fn residual_block_checked_constructor_accepts_dimensions_with_matching_row_counts() {
    let residuals = ResidualBlock::try_new(vec![vec![1.0, 2.0], vec![3.0, 4.0]])
        .expect("valid residual block should be accepted");

    assert_eq!(residuals.dimensions(), 2);
    assert_eq!(residuals.cardinality(), 2);
    assert!(residuals.has_consistent_shape());
    assert_eq!(residuals.dimension_lengths(), vec![2, 2]);
}

#[test]
fn residual_block_accepts_empty_internal_node_dimensions() {
    let residuals = ResidualBlock::new(vec![Vec::new(), Vec::new()]);

    assert_eq!(residuals.dimensions(), 2);
    assert_eq!(residuals.cardinality(), 0);
    assert!(residuals.is_empty());
    assert!(residuals.has_consistent_shape());
    assert_eq!(residuals.dimension_lengths(), vec![0, 0]);
}

#[test]
fn residual_block_checked_constructor_accepts_empty_internal_node_dimensions() {
    let residuals = ResidualBlock::try_new(vec![Vec::new(), Vec::new()])
        .expect("empty internal residual dimensions should be accepted");

    assert_eq!(residuals.dimensions(), 2);
    assert_eq!(residuals.cardinality(), 0);
    assert!(residuals.is_empty());
    assert!(residuals.has_consistent_shape());
    assert_eq!(residuals.dimension_lengths(), vec![0, 0]);
}

#[test]
fn residual_block_checked_constructor_reports_uneven_dimension_lengths() {
    let error = ResidualBlock::try_new(vec![vec![1.0, 2.0], vec![3.0]])
        .expect_err("uneven residual dimension lengths should be rejected");

    assert_eq!(
        error,
        ResidualBlockError::UnevenDimensionLength {
            dimension: 1,
            actual_rows: 1,
            expected_rows: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "residual dimension 1 has 1 rows but expected 2"
    );
}

#[test]
fn residual_block_checked_constructor_reports_nan_residual_value() {
    let error = ResidualBlock::try_new(vec![vec![1.0], vec![f32::NAN]])
        .expect_err("non-finite residual value should be rejected");

    assert_eq!(
        error,
        ResidualBlockError::NonFinite {
            dimension: 1,
            row: 0,
        }
    );
    assert_eq!(error.to_string(), "residual values must be finite");
}

#[test]
fn residual_block_checked_constructor_reports_infinite_residual_value() {
    let error = ResidualBlock::try_new(vec![vec![f32::INFINITY]])
        .expect_err("non-finite residual value should be rejected");

    assert_eq!(
        error,
        ResidualBlockError::NonFinite {
            dimension: 0,
            row: 0,
        }
    );
    assert_eq!(error.to_string(), "residual values must be finite");
}

#[test]
#[should_panic(expected = "residual dimension 1 has 1 rows but expected 2")]
fn residual_block_rejects_uneven_dimension_lengths() {
    let _residuals = ResidualBlock::new(vec![vec![1.0, 2.0], vec![3.0]]);
}

#[test]
#[should_panic(expected = "residual values must be finite")]
fn residual_block_rejects_nan_residual_value() {
    let _residuals = ResidualBlock::new(vec![vec![f32::NAN]]);
}

#[test]
#[should_panic(expected = "residual values must be finite")]
fn residual_block_rejects_infinite_residual_value() {
    let _residuals = ResidualBlock::new(vec![vec![f32::INFINITY]]);
}

#[test]
#[should_panic(expected = "centroid values must be finite")]
fn residual_block_from_points_rejects_non_finite_centroid() {
    let points = vec![Vector::new(vec![1.0])];

    let _residuals = ResidualBlock::from_points(&points, &[f32::NAN]);
}

#[test]
#[should_panic(expected = "point coordinates must be finite")]
fn residual_block_from_points_rejects_non_finite_point_coordinate() {
    let points = vec![Vector {
        values: vec![f32::NAN],
    }];

    let _residuals = ResidualBlock::from_points(&points, &[0.0]);
}

#[test]
fn bounding_box_contains_another_bounding_box() {
    let outer = BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let inner = BoundingBox::new(vec![2.0, 2.0], vec![8.0, 8.0]);

    assert!(outer.contains_bounds(&inner));
}

#[test]
fn bounding_box_does_not_contain_bounds_outside_its_range() {
    let outer = BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let outside = BoundingBox::new(vec![2.0, 2.0], vec![12.0, 8.0]);

    assert!(!outer.contains_bounds(&outside));
}
