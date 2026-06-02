use crate::query::{QueryRegion, QueryRegionError};

#[test]
fn query_region_accepts_valid_bounds() {
    let query = QueryRegion::new(vec![0.0, 1.0], vec![2.0, 3.0]);

    assert_eq!(query.min, vec![0.0, 1.0]);
    assert_eq!(query.max, vec![2.0, 3.0]);
    assert_eq!(query.dimensions(), 2);
}

#[test]
fn query_region_accepts_boundary_points() {
    let query = QueryRegion::new(vec![1.0, 2.0], vec![1.0, 2.0]);

    assert_eq!(query.min, vec![1.0, 2.0]);
    assert_eq!(query.max, vec![1.0, 2.0]);
}

#[test]
fn query_region_checked_constructor_accepts_valid_bounds() {
    let query = QueryRegion::try_new(vec![0.0, 1.0], vec![2.0, 3.0])
        .expect("valid query region should be accepted");

    assert_eq!(query.min, vec![0.0, 1.0]);
    assert_eq!(query.max, vec![2.0, 3.0]);
}

#[test]
fn query_region_checked_constructor_reports_mismatched_bound_dimensions() {
    let error = QueryRegion::try_new(vec![0.0, 1.0], vec![2.0])
        .expect_err("mismatched query bounds should be rejected");

    assert_eq!(
        error,
        QueryRegionError::DimensionMismatch {
            min_dimensions: 2,
            max_dimensions: 1,
        }
    );
    assert_eq!(
        error.to_string(),
        "query min and max vectors must have the same dimensionality"
    );
}

#[test]
fn query_region_checked_constructor_reports_empty_bounds() {
    let error = QueryRegion::try_new(Vec::new(), Vec::new())
        .expect_err("empty query bounds should be rejected");

    assert_eq!(error, QueryRegionError::Empty);
    assert_eq!(
        error.to_string(),
        "query region must have at least one dimension"
    );
}

#[test]
fn query_region_checked_constructor_reports_non_finite_bounds() {
    let error = QueryRegion::try_new(vec![0.0, f32::NAN], vec![1.0, 2.0])
        .expect_err("non-finite query bounds should be rejected");

    assert_eq!(error, QueryRegionError::NonFinite { dimension: 1 });
    assert_eq!(
        error.to_string(),
        "query bounds must be finite in every dimension"
    );
}

#[test]
fn query_region_checked_constructor_reports_inverted_bounds() {
    let error = QueryRegion::try_new(vec![0.0, 4.0], vec![1.0, 2.0])
        .expect_err("inverted query bounds should be rejected");

    assert_eq!(error, QueryRegionError::InvertedRange { dimension: 1 });
    assert_eq!(
        error.to_string(),
        "query minimum must not exceed maximum in dimension 1"
    );
}

#[test]
#[should_panic(expected = "query min and max vectors must have the same dimensionality")]
fn query_region_rejects_mismatched_bound_dimensions() {
    let _ = QueryRegion::new(vec![0.0, 1.0], vec![2.0]);
}

#[test]
#[should_panic(expected = "query region must have at least one dimension")]
fn query_region_rejects_empty_bounds() {
    let _ = QueryRegion::new(Vec::new(), Vec::new());
}

#[test]
#[should_panic(expected = "query minimum must not exceed maximum in dimension 0")]
fn query_region_rejects_inverted_bounds() {
    let _ = QueryRegion::new(vec![3.0, 0.0], vec![2.0, 1.0]);
}

#[test]
#[should_panic(expected = "query bounds must be finite in every dimension")]
fn query_region_rejects_nan_minimum() {
    let _ = QueryRegion::new(vec![f32::NAN], vec![1.0]);
}

#[test]
#[should_panic(expected = "query bounds must be finite in every dimension")]
fn query_region_rejects_infinite_maximum() {
    let _ = QueryRegion::new(vec![0.0], vec![f32::INFINITY]);
}
