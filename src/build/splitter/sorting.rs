//! Splitter point ordering helpers.

use std::cmp::Ordering;

use crate::math::Vector;

pub(super) fn sorted_points_on_axis(points: &[Vector], split_dimension: usize) -> Vec<Vector> {
    let mut sorted = points.to_vec();

    // full sort is still the obvious version
    // select_nth_unstable can come later if this shows up in profiles
    sorted.sort_by(|left, right| {
        left.values[split_dimension]
            .partial_cmp(&right.values[split_dimension])
            .unwrap_or(Ordering::Equal)
    });

    sorted
}

pub(super) fn split_sorted_points_at_index(
    mut sorted: Vec<Vector>,
    split_index: usize,
) -> (Vec<Vector>, Vec<Vector>) {
    assert!(
        split_index > 0 && split_index < sorted.len(),
        "split index must produce two non-empty sides"
    );

    let right = sorted.split_off(split_index);
    let left = sorted;

    assert!(!left.is_empty(), "split produced an empty left side");
    assert!(!right.is_empty(), "split produced an empty right side");

    (left, right)
}
