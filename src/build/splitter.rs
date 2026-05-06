//! Point splitting utilities for index construction.

use crate::build::variance::max_variance_dimension;
use crate::math::Vector;

/// Selects the split axis for a point set.
///
/// # Runtime Role
///
/// The initial implementation chooses the dimension with maximum empirical
/// variance. This keeps the split deterministic while giving the builder a
/// simple geometry-aware partitioning rule.
///
/// # Panics
///
/// Panics when the point set is empty or dimensionality is inconsistent.
pub fn select_split_axis(points: &[Vector]) -> usize {
    max_variance_dimension(points)
}

/// Splits points at the median along the selected dimension.
///
/// # Runtime Role
///
/// This performs the physical point split after a split axis has already been
/// selected.
///
/// # Panics
///
/// Panics when fewer than two points are provided.
pub fn median_split_on_axis(
    points: &[Vector],
    split_dimension: usize,
) -> (Vec<Vector>, Vec<Vector>) {
    assert!(
        points.len() >= 2,
        "median split requires at least two points"
    );

    let mut sorted = points.to_vec();

    // sort_by is O(N log N)
    // we technically only need to find the median,
    // so `select_nth_unstable` (O(N)) would be strictly better here.
    sorted.sort_by(|left, right| {
        left.values[split_dimension]
            .partial_cmp(&right.values[split_dimension])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let midpoint = sorted.len() / 2;
    let right = sorted.split_off(midpoint);
    let left = sorted;

    assert!(!left.is_empty(), "median split produced an empty left side");
    assert!(
        !right.is_empty(),
        "median split produced an empty right side"
    );

    (left, right)
}

/// Splits points at the median along the maximum-variance dimension.
///
/// # Runtime Role
///
/// This convenience function preserves the original builder behavior while
/// delegating split-axis selection and median splitting to separate functions.
pub fn median_split(points: &[Vector]) -> (Vec<Vector>, Vec<Vector>) {
    let split_dimension = select_split_axis(points);
    median_split_on_axis(points, split_dimension)
}
