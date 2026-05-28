//! Split scoring helpers.

use std::cmp::Ordering;

use crate::build::metrics::{
    bounds_overlap_extent_sum_prevalidated, split_quality_metrics_from_bounds,
};
use crate::math::{BoundingBox, Scalar, Vector};

use super::{MedianSplit, SplitAxisScore, median_split_on_axis, structural_split_on_axis};

pub(super) fn median_split_on_axis_with_variance(
    points: &[Vector],
    split_dimension: usize,
    variance: Scalar,
) -> MedianSplit {
    let (left_points, right_points) = median_split_on_axis(points, split_dimension);
    split_with_score(points, split_dimension, variance, left_points, right_points)
}

pub(super) fn structural_split_on_axis_with_variance(
    points: &[Vector],
    split_dimension: usize,
    variance: Scalar,
) -> MedianSplit {
    let (left_points, right_points) = structural_split_on_axis(points, split_dimension);
    split_with_score(points, split_dimension, variance, left_points, right_points)
}

fn split_with_score(
    parent_points: &[Vector],
    split_dimension: usize,
    variance: Scalar,
    left_points: Vec<Vector>,
    right_points: Vec<Vector>,
) -> MedianSplit {
    let parent_bounds = BoundingBox::from_points(parent_points);
    let left_bounds = BoundingBox::from_points(&left_points);
    let right_bounds = BoundingBox::from_points(&right_points);

    let metrics = split_quality_metrics_from_bounds(
        &parent_bounds,
        &left_bounds,
        &right_bounds,
        parent_points.len(),
        left_points.len(),
        right_points.len(),
    );

    let child_overlap_extent = child_overlap_extent_sum(&left_bounds, &right_bounds);

    // metrics owns the geometry now
    let score = SplitAxisScore {
        split_dimension,
        metrics,
        child_overlap_extent,
        variance,
    };

    MedianSplit {
        score,
        left_points,
        right_points,
    }
}

fn child_overlap_extent_sum(left_bounds: &BoundingBox, right_bounds: &BoundingBox) -> Scalar {
    debug_assert_eq!(
        left_bounds.dimensions(),
        right_bounds.dimensions(),
        "child bounds should have matching dimensionality"
    );

    bounds_overlap_extent_sum_prevalidated(left_bounds, right_bounds)
}

pub(super) fn compare_split_axis_scores(left: &SplitAxisScore, right: &SplitAxisScore) -> Ordering {
    compare_scalar(left.combined_child_volume(), right.combined_child_volume())
        .then_with(|| compare_scalar(left.child_overlap_extent(), right.child_overlap_extent()))
        .then_with(|| {
            compare_scalar(
                right.volume_reduction_ratio(),
                left.volume_reduction_ratio(),
            )
        })
        .then_with(|| compare_scalar(left.combined_child_extent(), right.combined_child_extent()))
        .then_with(|| {
            compare_scalar(
                right.extent_reduction_ratio(),
                left.extent_reduction_ratio(),
            )
        })
        .then_with(|| left.balance_penalty().cmp(&right.balance_penalty()))
        .then_with(|| compare_scalar(right.variance, left.variance))
        .then_with(|| left.split_dimension.cmp(&right.split_dimension))
}

fn compare_scalar(left: Scalar, right: Scalar) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}
