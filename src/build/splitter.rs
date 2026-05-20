//! Point splitting utilities for index construction.

use std::cmp::Ordering;

use crate::build::metrics::{SplitQualityMetrics, split_quality_metrics};
use crate::build::variance::variance_by_dimension;
use crate::math::{BoundingBox, Scalar, Vector};

const STRUCTURAL_GAP_DOMINANCE_RATIO: Scalar = 4.0;

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

    /// Structural quality metrics for the split along this axis.
    pub metrics: SplitQualityMetrics,

    /// Sum of overlapping child extents across dimensions.
    ///
    /// # Runtime Role
    ///
    /// This estimates sibling overlap pressure. Lower overlap is preferred when
    /// candidate splits otherwise have equivalent child volume because overlapping
    /// child bounds can cause the same query to retain both children.
    pub child_overlap_extent: Scalar,

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

    /// Returns sibling overlap pressure measured as summed overlapping extent.
    pub fn child_overlap_extent(&self) -> Scalar {
        self.child_overlap_extent
    }
}

/// Point split result paired with the score that selected it.
///
/// # Runtime Role
///
/// The builder needs both the chosen split metadata and the child point sets.
/// Keeping them together avoids scoring the selected axis and then sorting the
/// same points again to produce the actual children.
#[derive(Clone, Debug, PartialEq)]
pub struct MedianSplit {
    /// Score for the selected split axis.
    pub score: SplitAxisScore,

    /// Points routed to the left child partition.
    pub left_points: Vec<Vector>,

    /// Points routed to the right child partition.
    pub right_points: Vec<Vector>,
}

impl MedianSplit {
    /// Returns the selected split dimension.
    pub fn split_dimension(&self) -> usize {
        self.score.split_dimension
    }

    /// Returns structural quality metrics for the selected split.
    pub fn metrics(&self) -> SplitQualityMetrics {
        self.score.metrics
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

fn median_split_on_axis_with_variance(
    points: &[Vector],
    split_dimension: usize,
    variance: Scalar,
) -> MedianSplit {
    let (left_points, right_points) = median_split_on_axis(points, split_dimension);
    split_with_score(points, split_dimension, variance, left_points, right_points)
}

fn structural_split_on_axis_with_variance(
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
    let metrics = split_quality_metrics(parent_points, &left_points, &right_points);
    let child_overlap_extent = child_overlap_extent_sum(&left_points, &right_points);

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

fn child_overlap_extent_sum(left_points: &[Vector], right_points: &[Vector]) -> Scalar {
    let left_bounds = BoundingBox::from_points(left_points);
    let right_bounds = BoundingBox::from_points(right_points);

    let mut overlap_extent = 0.0;

    for dimension in 0..left_bounds.dimensions() {
        let overlap_min = left_bounds.min[dimension].max(right_bounds.min[dimension]);
        let overlap_max = left_bounds.max[dimension].min(right_bounds.max[dimension]);
        let overlap_width = overlap_max - overlap_min;

        if overlap_width < 0.0 {
            return 0.0;
        }

        overlap_extent += overlap_width;
    }

    overlap_extent
}

fn sorted_points_on_axis(points: &[Vector], split_dimension: usize) -> Vec<Vector> {
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

fn guarded_structural_split_index(sorted: &[Vector], split_dimension: usize) -> usize {
    structural_gap_split_index(sorted, split_dimension).unwrap_or(sorted.len() / 2)
}

fn structural_gap_split_index(sorted: &[Vector], split_dimension: usize) -> Option<usize> {
    debug_assert!(
        sorted.len() >= 2,
        "structural gap split requires at least two sorted points"
    );

    let median_index = sorted.len() / 2;
    let mut best_split_index = median_index;
    let mut best_gap = Scalar::NEG_INFINITY;
    let mut best_median_distance = usize::MAX;
    let mut positive_gaps = Vec::with_capacity(sorted.len().saturating_sub(1));

    for split_index in 1..sorted.len() {
        let previous_value = sorted[split_index - 1].values[split_dimension];
        let next_value = sorted[split_index].values[split_dimension];
        let gap = next_value - previous_value;

        if gap > 0.0 {
            positive_gaps.push(gap);
        }

        let median_distance = split_index.abs_diff(median_index);

        if gap > best_gap || (gap == best_gap && median_distance < best_median_distance) {
            best_gap = gap;
            best_split_index = split_index;
            best_median_distance = median_distance;
        }
    }

    if !gap_is_structural(best_gap, &mut positive_gaps) {
        return None;
    }

    Some(best_split_index)
}

fn gap_is_structural(largest_gap: Scalar, positive_gaps: &mut Vec<Scalar>) -> bool {
    if largest_gap <= 0.0 || positive_gaps.is_empty() {
        return false;
    }

    if positive_gaps.len() == 1 {
        return true;
    }

    positive_gaps.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));

    let local_spacing = lower_median_positive_gap(positive_gaps);

    if local_spacing <= 0.0 {
        return true;
    }

    // largest gap has to stand out from normal local spacing
    largest_gap >= local_spacing * STRUCTURAL_GAP_DOMINANCE_RATIO
}

fn lower_median_positive_gap(sorted_positive_gaps: &[Scalar]) -> Scalar {
    let median_index = (sorted_positive_gaps.len() - 1) / 2;

    sorted_positive_gaps[median_index]
}

fn split_sorted_points_at_index(
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

fn compare_split_axis_scores(left: &SplitAxisScore, right: &SplitAxisScore) -> Ordering {
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
