use crate::query::QueryRegion;

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
