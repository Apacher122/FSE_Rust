use crate::math::{BoundingBox, ResidualBlock, Vector};

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
