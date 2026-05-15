use std::cmp::Ordering;

use crate::benchmark::{compare_points_lexicographically, sort_points_lexicographically};
use crate::math::Vector;

#[test]
fn sort_points_lexicographically_orders_by_coordinate_values() {
    let mut points = vec![
        Vector::new(vec![2.0, 0.0]),
        Vector::new(vec![1.0, 2.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![0.0, 9.0]),
    ];

    sort_points_lexicographically(&mut points);

    assert_eq!(
        points,
        vec![
            Vector::new(vec![0.0, 9.0]),
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![1.0, 2.0]),
            Vector::new(vec![2.0, 0.0]),
        ]
    );
}

#[test]
fn compare_points_lexicographically_falls_back_to_vector_length() {
    let shorter = Vector::new(vec![1.0, 2.0]);
    let longer = Vector::new(vec![1.0, 2.0, 3.0]);

    assert_eq!(
        compare_points_lexicographically(&shorter, &longer),
        Ordering::Less
    );

    assert_eq!(
        compare_points_lexicographically(&longer, &shorter),
        Ordering::Greater
    );
}

#[test]
fn compare_points_lexicographically_treats_unordered_scalar_positions_as_equal() {
    let left = Vector::new(vec![f32::NAN]);
    let right = Vector::new(vec![0.0]);

    assert_eq!(
        compare_points_lexicographically(&left, &right),
        Ordering::Equal
    );
}

#[test]
fn compare_points_lexicographically_continues_after_unordered_scalar_positions() {
    let left = Vector::new(vec![f32::NAN, 1.0]);
    let right = Vector::new(vec![0.0, 2.0]);

    assert_eq!(
        compare_points_lexicographically(&left, &right),
        Ordering::Less
    );
}
