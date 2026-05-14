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
