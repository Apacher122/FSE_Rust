use crate::build::splitter::select_split_axis;
use crate::math::Vector;

#[test]
fn select_split_axis_chooses_dimension_with_highest_variance() {
    let points = vec![
        Vector::new(vec![0.0, 1.0]),
        Vector::new(vec![10.0, 2.0]),
        Vector::new(vec![20.0, 3.0]),
    ];

    let axis = select_split_axis(&points);

    assert_eq!(axis, 0);
}

#[test]
fn median_split_on_axis_splits_points_into_two_non_empty_groups() {
    let points = vec![
        Vector::new(vec![3.0, 0.0]),
        Vector::new(vec![1.0, 0.0]),
        Vector::new(vec![4.0, 0.0]),
        Vector::new(vec![2.0, 0.0]),
    ];

    let (left, right) = median_split_on_axis(&points, 0);

    assert_eq!(
        left,
        vec![Vector::new(vec![1.0, 0.0]), Vector::new(vec![2.0, 0.0]),]
    );

    assert_eq!(
        right,
        vec![Vector::new(vec![3.0, 0.0]), Vector::new(vec![4.0, 0.0]),]
    );
}
