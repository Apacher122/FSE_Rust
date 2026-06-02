use crate::math::{CentroidError, Vector, compute_centroid, try_compute_centroid};

#[test]
fn centroid_is_mean_of_points_by_dimension() {
    let points = vec![
        Vector::new(vec![0.0, 2.0]),
        Vector::new(vec![2.0, 4.0]),
        Vector::new(vec![4.0, 6.0]),
    ];
    let centroid = compute_centroid(&points);
    assert_eq!(centroid, vec![2.0, 4.0]);
}

#[test]
fn checked_centroid_is_mean_of_points_by_dimension() {
    let points = vec![
        Vector::new(vec![0.0, 2.0]),
        Vector::new(vec![2.0, 4.0]),
        Vector::new(vec![4.0, 6.0]),
    ];
    let centroid = try_compute_centroid(&points).expect("valid points should produce a centroid");
    assert_eq!(centroid, vec![2.0, 4.0]);
}

#[test]
fn centroid_of_single_point_is_that_point() {
    let points = vec![Vector::new(vec![3.0, 7.0])];
    let centroid = compute_centroid(&points);
    assert_eq!(centroid, vec![3.0, 7.0]);
}

#[test]
fn checked_centroid_reports_empty_point_set() {
    let error = try_compute_centroid(&[]).expect_err("empty point set should be rejected");

    assert_eq!(error, CentroidError::EmptyPointSet);
    assert_eq!(
        error.to_string(),
        "cannot compute centroid for empty points"
    );
}

#[test]
fn checked_centroid_reports_empty_point_dimensions() {
    let points = vec![Vector { values: Vec::new() }];

    let error =
        try_compute_centroid(&points).expect_err("empty point dimensions should be rejected");

    assert_eq!(error, CentroidError::EmptyPointDimensions);
    assert_eq!(error.to_string(), "points must have at least one dimension");
}

#[test]
fn checked_centroid_reports_dimension_mismatch() {
    let points = vec![Vector::new(vec![1.0, 2.0]), Vector::new(vec![3.0])];

    let error =
        try_compute_centroid(&points).expect_err("mismatched point dimensions should be rejected");

    assert_eq!(
        error,
        CentroidError::DimensionMismatch {
            point: 1,
            actual_dimensions: 1,
            expected_dimensions: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "all points must have the same dimensionality"
    );
}

#[test]
fn checked_centroid_reports_non_finite_point_coordinate() {
    let points = vec![Vector {
        values: vec![f32::NAN],
    }];

    let error =
        try_compute_centroid(&points).expect_err("non-finite point coordinate should be rejected");

    assert_eq!(
        error,
        CentroidError::NonFiniteCoordinate {
            point: 0,
            dimension: 0,
        }
    );
    assert_eq!(error.to_string(), "point coordinates must be finite");
}

#[test]
fn checked_centroid_reports_non_finite_computed_value() {
    let points = vec![Vector::new(vec![f32::MAX]), Vector::new(vec![f32::MAX])];

    let error = try_compute_centroid(&points).expect_err("non-finite centroid should be rejected");

    assert_eq!(error, CentroidError::NonFiniteCentroid { dimension: 0 });
    assert_eq!(error.to_string(), "centroid values must be finite");
}

#[test]
#[should_panic(expected = "point coordinates must be finite")]
fn centroid_rejects_non_finite_point_coordinate() {
    let points = vec![Vector {
        values: vec![f32::NAN],
    }];

    let _centroid = compute_centroid(&points);
}

#[test]
#[should_panic(expected = "centroid values must be finite")]
fn centroid_rejects_non_finite_computed_value() {
    let points = vec![Vector::new(vec![f32::MAX]), Vector::new(vec![f32::MAX])];

    let _centroid = compute_centroid(&points);
}
