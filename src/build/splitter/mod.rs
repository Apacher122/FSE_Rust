//! Point splitting utilities for index construction.

mod scoring;
mod sorting;
mod structural;
mod types;
mod validation;

use crate::build::variance::variance_by_dimension;
use crate::math::Vector;

use scoring::{
    compare_split_axis_scores, median_split_on_axis_with_variance,
    structural_split_on_axis_with_variance,
};
use sorting::{sorted_points_on_axis, split_sorted_points_at_index};
use structural::guarded_structural_split_index;
pub use types::{MedianSplit, SplitAxisScore};
use validation::validate_points_for_split;

/// Selects the split axis for a point set.
///
/// # Runtime Role
///
/// The split axis is selected by evaluating every median split and choosing the
/// one that minimizes combined child bounding volume. This directly targets
/// tighter child partitions instead of assuming the highest-variance axis always
/// produces the best pruning geometry.
///
/// # Panics
///
/// Panics when fewer than two points are provided or dimensionality is
/// inconsistent.
pub fn select_split_axis(points: &[Vector]) -> usize {
    best_median_split_axis_score(points).split_dimension
}

/// Returns the best median split-axis score for a point set.
///
/// # Runtime Role
///
/// This function evaluates all dimensions as median split candidates and ranks
/// them by expected child bounding tightness.
///
/// The ordering is:
///
/// 1. Lower combined child bounding volume.
/// 2. Lower sibling overlap extent.
/// 3. Higher volume reduction ratio.
/// 4. Lower combined child extent.
/// 5. Higher extent reduction ratio.
/// 6. Lower balance penalty.
/// 7. Higher variance.
/// 8. Lower dimension index.
///
/// # Panics
///
/// Panics when fewer than two points are provided or dimensionality is
/// inconsistent.
pub fn best_median_split_axis_score(points: &[Vector]) -> SplitAxisScore {
    best_median_split(points).score
}

/// Returns the best median split for a point set.
///
/// # Runtime Role
///
/// This helper evaluates candidate median split axes and returns the winning
/// score together with the already-sorted child point sets.
///
/// # Panics
///
/// Panics when fewer than two points are provided or dimensionality is
/// inconsistent.
pub fn best_median_split(points: &[Vector]) -> MedianSplit {
    let dimensions = validate_points_for_split(points);
    let variances = variance_by_dimension(points);

    (0..dimensions)
        .map(|split_dimension| {
            median_split_on_axis_with_variance(points, split_dimension, variances[split_dimension])
        })
        .min_by(|left, right| compare_split_axis_scores(&left.score, &right.score))
        .expect("validated split input should have at least one dimension")
}

/// Returns the best guarded structural split for a point set.
///
/// # Runtime Role
///
/// This is the builder-facing split helper. It evaluates every axis using a
/// guarded structural split:
///
/// - When an axis contains a dominant coordinate gap, split at that gap.
/// - Otherwise, fall back to median splitting on that axis.
///
/// This preserves cluster separation for clearly separated groups without
/// overreacting to uniform spacing.
///
/// # Panics
///
/// Panics when fewer than two points are provided or dimensionality is
/// inconsistent.
pub fn best_structural_split(points: &[Vector]) -> MedianSplit {
    let dimensions = validate_points_for_split(points);
    let variances = variance_by_dimension(points);

    (0..dimensions)
        .map(|split_dimension| {
            structural_split_on_axis_with_variance(
                points,
                split_dimension,
                variances[split_dimension],
            )
        })
        .min_by(|left, right| compare_split_axis_scores(&left.score, &right.score))
        .expect("validated split input should have at least one dimension")
}

/// Scores a median split along one dimension.
///
/// # Runtime Role
///
/// This exposes the split scoring rule for tests and future builder tuning.
/// It does not mutate the input point set.
///
/// # Panics
///
/// Panics when fewer than two points are provided, when the split dimension is
/// out of range, or when dimensionality is inconsistent.
pub fn median_split_score_on_axis(points: &[Vector], split_dimension: usize) -> SplitAxisScore {
    let variances = variance_by_dimension(points);

    assert!(
        split_dimension < variances.len(),
        "split dimension must be inside point dimensionality"
    );

    median_split_on_axis_with_variance(points, split_dimension, variances[split_dimension]).score
}

/// Scores a guarded structural split along one dimension.
///
/// # Runtime Role
///
/// This exposes the builder-facing split rule for tests. It uses the largest
/// structural gap only when that gap dominates local spacing; otherwise it
/// scores the median split on the selected axis.
///
/// # Panics
///
/// Panics when fewer than two points are provided, when the split dimension is
/// out of range, or when dimensionality is inconsistent.
pub fn structural_split_score_on_axis(points: &[Vector], split_dimension: usize) -> SplitAxisScore {
    let variances = variance_by_dimension(points);

    assert!(
        split_dimension < variances.len(),
        "split dimension must be inside point dimensionality"
    );

    structural_split_on_axis_with_variance(points, split_dimension, variances[split_dimension])
        .score
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
/// Panics when fewer than two points are provided, when the split dimension is
/// out of range, or when dimensionality is inconsistent.
pub fn median_split_on_axis(
    points: &[Vector],
    split_dimension: usize,
) -> (Vec<Vector>, Vec<Vector>) {
    let dimensions = validate_points_for_split(points);

    assert!(
        split_dimension < dimensions,
        "split dimension must be inside point dimensionality"
    );

    let sorted = sorted_points_on_axis(points, split_dimension);
    split_sorted_points_at_index(sorted, points.len() / 2)
}

/// Splits points using the guarded structural split rule along one dimension.
///
/// # Runtime Role
///
/// This is useful for tests and diagnostics. The builder normally calls
/// [`best_structural_split`] so every axis can compete by split quality.
///
/// # Panics
///
/// Panics when fewer than two points are provided, when the split dimension is
/// out of range, or when dimensionality is inconsistent.
pub fn structural_split_on_axis(
    points: &[Vector],
    split_dimension: usize,
) -> (Vec<Vector>, Vec<Vector>) {
    let dimensions = validate_points_for_split(points);

    assert!(
        split_dimension < dimensions,
        "split dimension must be inside point dimensionality"
    );

    let sorted = sorted_points_on_axis(points, split_dimension);
    let split_index = guarded_structural_split_index(&sorted, split_dimension);
    split_sorted_points_at_index(sorted, split_index)
}

/// Splits points at the median along the best geometric split dimension.
///
/// # Runtime Role
///
/// This convenience function preserves the median split API while selecting the
/// split dimension with the shared split-quality metric definition.
pub fn median_split(points: &[Vector]) -> (Vec<Vector>, Vec<Vector>) {
    let split = best_median_split(points);

    (split.left_points, split.right_points)
}
