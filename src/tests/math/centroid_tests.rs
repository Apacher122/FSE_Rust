use crate::math::{Vector, compute_centroid};

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
fn centroid_of_single_point_is_that_point() {
    let points = vec![Vector::new(vec![3.0, 7.0])];
    let centroid = compute_centroid(&points);
    assert_eq!(centroid, vec![3.0, 7.0]);
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
