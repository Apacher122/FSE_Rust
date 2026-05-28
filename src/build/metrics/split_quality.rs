//! Split quality metrics.

use crate::build::splitter::median_split_on_axis;
use crate::math::{BoundingBox, Scalar, Vector};

use super::types::SplitQualityMetrics;

/// Computes split quality metrics from parent and child point sets.
///
/// # Runtime Role
///
/// This helper evaluates the geometric effect of a split after the child point
/// sets are known. It is useful for testing split policies and for future build
/// heuristics that need to compare candidate subdivisions.
///
/// # Panics
///
/// Panics when any point set is empty, when dimensionality is inconsistent, or
/// when child cardinalities do not add up to parent cardinality.
pub fn split_quality_metrics(
    parent_points: &[Vector],
    left_points: &[Vector],
    right_points: &[Vector],
) -> SplitQualityMetrics {
    validate_split_point_sets(parent_points, left_points, right_points);

    let parent_bounds = BoundingBox::from_points(parent_points);
    let left_bounds = BoundingBox::from_points(left_points);
    let right_bounds = BoundingBox::from_points(right_points);

    split_quality_metrics_from_bounds(
        &parent_bounds,
        &left_bounds,
        &right_bounds,
        parent_points.len(),
        left_points.len(),
        right_points.len(),
    )
}

/// Computes split quality metrics for a median split along one axis.
///
/// # Runtime Role
///
/// This helper measures the child-volume effect of one candidate median split
/// axis without changing the builder. It is intended for split heuristic tests
/// and future build tuning.
///
/// # Panics
///
/// Panics when fewer than two points are provided, when the split dimension is
/// out of range, or when dimensionality is inconsistent.
pub fn split_quality_metrics_for_axis(
    points: &[Vector],
    split_dimension: usize,
) -> SplitQualityMetrics {
    let (left, right) = median_split_on_axis(points, split_dimension);

    // tiny helper but this is the metric we actually care about
    split_quality_metrics(points, &left, &right)
}

/// Computes split quality metrics from already computed bounds.
///
/// # Runtime Role
///
/// This helper avoids recomputing bounding boxes when callers already have the
/// parent and child support regions available.
///
/// # Panics
///
/// Panics when bounding dimensionality is inconsistent or when child
/// cardinalities do not add up to parent cardinality.
pub fn split_quality_metrics_from_bounds(
    parent_bounds: &BoundingBox,
    left_bounds: &BoundingBox,
    right_bounds: &BoundingBox,
    parent_cardinality: usize,
    left_cardinality: usize,
    right_cardinality: usize,
) -> SplitQualityMetrics {
    assert_eq!(
        parent_bounds.dimensions(),
        left_bounds.dimensions(),
        "left child bounds must match parent dimensionality"
    );
    assert_eq!(
        parent_bounds.dimensions(),
        right_bounds.dimensions(),
        "right child bounds must match parent dimensionality"
    );
    assert_eq!(
        left_cardinality + right_cardinality,
        parent_cardinality,
        "child cardinalities must add up to parent cardinality"
    );

    let parent_volume = parent_bounds.volume();
    let combined_child_volume = left_bounds.volume() + right_bounds.volume();
    let volume_reduction_ratio = reduction_ratio(parent_volume, combined_child_volume);

    let parent_extent = bounding_extent_sum(parent_bounds);
    let combined_child_extent =
        bounding_extent_sum(left_bounds) + bounding_extent_sum(right_bounds);
    let extent_reduction_ratio = reduction_ratio(parent_extent, combined_child_extent);

    let balance_penalty = left_cardinality.abs_diff(right_cardinality);

    SplitQualityMetrics {
        parent_volume,
        combined_child_volume,
        volume_reduction_ratio,
        parent_extent,
        combined_child_extent,
        extent_reduction_ratio,
        parent_cardinality,
        left_cardinality,
        right_cardinality,
        balance_penalty,
    }
}

/// Returns the sum of bounding widths across dimensions.
///
/// # Runtime Role
///
/// Extent is a fallback quality signal when volume collapses to zero because one
/// or more dimensions are degenerate.
pub fn bounding_extent_sum(bounds: &BoundingBox) -> Scalar {
    bounds
        .min
        .iter()
        .zip(&bounds.max)
        .map(|(minimum, maximum)| (maximum - minimum).max(0.0))
        .sum()
}

fn reduction_ratio(parent_value: Scalar, child_value: Scalar) -> Scalar {
    if parent_value <= 0.0 {
        return 0.0;
    }

    // can go negative when child boxes overlap too much
    (parent_value - child_value) / parent_value
}

fn validate_split_point_sets(
    parent_points: &[Vector],
    left_points: &[Vector],
    right_points: &[Vector],
) {
    assert!(
        !parent_points.is_empty(),
        "parent point set must not be empty"
    );
    assert!(!left_points.is_empty(), "left point set must not be empty");
    assert!(
        !right_points.is_empty(),
        "right point set must not be empty"
    );
    assert_eq!(
        left_points.len() + right_points.len(),
        parent_points.len(),
        "child point counts must add up to parent point count"
    );

    let dimensions = parent_points[0].dimensions();
    assert!(dimensions > 0, "points must have at least one dimension");

    // boring but worth catching before metrics lie to us
    for point in parent_points.iter().chain(left_points).chain(right_points) {
        assert_eq!(
            point.dimensions(),
            dimensions,
            "all split metric points must have the same dimensionality"
        );
    }
}
