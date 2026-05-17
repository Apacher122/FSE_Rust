//! Point splitting utilities for index construction.

use std::cmp::Ordering;

use crate::build::metrics::{SplitQualityMetrics, split_quality_metrics_for_axis};
use crate::build::variance::variance_by_dimension;
use crate::math::{Scalar, Vector};

/// Split-axis score used during partition construction.
///
/// # Runtime Role
///
/// `SplitAxisScore` records the geometric quality of splitting a point set along
/// one dimension. The score delegates structural geometry measurements to
/// `SplitQualityMetrics` so split selection and split diagnostics use the same
/// definition of child volume, extent, and balance.
///
/// # Formal Reference
///
/// This supports density-aware subdivision by ranking candidate split axes using
/// the volumetric improvement of their child support regions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitAxisScore {
    /// Candidate split dimension.
    pub split_dimension: usize,

    /// Structural quality metrics for the median split along this axis.
    pub metrics: SplitQualityMetrics,

    /// Variance of the selected split dimension.
    pub variance: Scalar,
}

impl SplitAxisScore {
    /// Returns the sum of left and right child bounding volumes.
    pub fn combined_child_volume(&self) -> Scalar {
        self.metrics.combined_child_volume
    }

    /// Returns the relative volume reduction from parent to children.
    pub fn volume_reduction_ratio(&self) -> Scalar {
        self.metrics.volume_reduction_ratio
    }

    /// Returns the sum of left and right child bounding extents.
    pub fn combined_child_extent(&self) -> Scalar {
        self.metrics.combined_child_extent
    }

    /// Returns the relative extent reduction from parent to children.
    pub fn extent_reduction_ratio(&self) -> Scalar {
        self.metrics.extent_reduction_ratio
    }

    /// Returns the absolute difference between child cardinalities.
    pub fn balance_penalty(&self) -> usize {
        self.metrics.balance_penalty
    }
}

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
/// 2. Higher volume reduction ratio.
/// 3. Lower combined child extent.
/// 4. Higher extent reduction ratio.
/// 5. Lower balance penalty.
/// 6. Higher variance.
/// 7. Lower dimension index.
///
/// # Panics
///
/// Panics when fewer than two points are provided or dimensionality is
/// inconsistent.
pub fn best_median_split_axis_score(points: &[Vector]) -> SplitAxisScore {
    let dimensions = validate_points_for_split(points);
    let variances = variance_by_dimension(points);

    // one scoring source or this will rot later
    (0..dimensions)
        .map(|split_dimension| {
            median_split_score_on_axis_with_variance(
                points,
                split_dimension,
                variances[split_dimension],
            )
        })
        .min_by(compare_split_axis_scores)
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

    median_split_score_on_axis_with_variance(points, split_dimension, variances[split_dimension])
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

    let mut sorted = points.to_vec();

    // full sort is still the obvious version
    // select_nth_unstable can come later if this shows up in profiles
    sorted.sort_by(|left, right| {
        left.values[split_dimension]
            .partial_cmp(&right.values[split_dimension])
            .unwrap_or(Ordering::Equal)
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

/// Splits points at the median along the best geometric split dimension.
///
/// # Runtime Role
///
/// This convenience function preserves the builder API while selecting the split
/// dimension with the shared split-quality metric definition.
pub fn median_split(points: &[Vector]) -> (Vec<Vector>, Vec<Vector>) {
    let split_dimension = select_split_axis(points);
    median_split_on_axis(points, split_dimension)
}

fn median_split_score_on_axis_with_variance(
    points: &[Vector],
    split_dimension: usize,
    variance: Scalar,
) -> SplitAxisScore {
    let metrics = split_quality_metrics_for_axis(points, split_dimension);

    // metrics owns the geometry now
    SplitAxisScore {
        split_dimension,
        metrics,
        variance,
    }
}

fn compare_split_axis_scores(left: &SplitAxisScore, right: &SplitAxisScore) -> Ordering {
    compare_scalar(left.combined_child_volume(), right.combined_child_volume())
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

fn validate_points_for_split(points: &[Vector]) -> usize {
    assert!(
        points.len() >= 2,
        "median split requires at least two points"
    );

    let dimensions = points[0].dimensions();
    assert!(dimensions > 0, "points must have at least one dimension");

    for point in points {
        assert_eq!(
            point.dimensions(),
            dimensions,
            "all points must have the same dimensionality"
        );
    }

    dimensions
}
