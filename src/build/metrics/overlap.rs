//! Sibling-overlap metrics.

use crate::math::{BoundingBox, Scalar};
use crate::storage::FSEIndex;

use super::types::SiblingOverlapMetrics;

/// Computes sibling-overlap metrics for an FSE hierarchy.
///
/// # Runtime Role
///
/// This diagnostic helper walks internal hierarchy nodes and measures the
/// overlap extent between sibling child bounds. It does not affect split
/// selection or query execution.
///
/// # Formal Reference
///
/// Sibling overlap estimates one source of geometric over-retention during
/// Stage I traversal.
pub fn sibling_overlap_metrics(index: &FSEIndex) -> SiblingOverlapMetrics {
    let mut sibling_pair_count = 0;
    let mut overlapping_sibling_pair_count = 0;
    let mut total_overlap_extent = 0.0;

    for node in &index.nodes {
        if node.is_leaf || node.children.len() < 2 {
            continue;
        }

        for left_child_offset in 0..node.children.len() {
            for right_child_offset in (left_child_offset + 1)..node.children.len() {
                let left_child_id = node.children[left_child_offset];
                let right_child_id = node.children[right_child_offset];

                let left_bounds = &index.nodes[left_child_id].bounds;
                let right_bounds = &index.nodes[right_child_id].bounds;
                let overlap_extent = sibling_overlap_extent_sum(left_bounds, right_bounds);

                sibling_pair_count += 1;
                total_overlap_extent += overlap_extent;

                if overlap_extent > 0.0 {
                    overlapping_sibling_pair_count += 1;
                }
            }
        }
    }

    let average_overlap_extent = if sibling_pair_count == 0 {
        0.0
    } else {
        total_overlap_extent / sibling_pair_count as Scalar
    };

    SiblingOverlapMetrics {
        sibling_pair_count,
        overlapping_sibling_pair_count,
        total_overlap_extent,
        average_overlap_extent,
    }
}

/// Computes summed overlap extent between two sibling bounds.
///
/// # Runtime Role
///
/// The returned value is zero when bounds are disjoint in any dimension.
/// Otherwise, it sums the overlapping width along each dimension. This is a
/// lightweight pressure metric rather than a volume metric, so it still has a
/// useful signal when one dimension is degenerate.
///
/// # Panics
///
/// Panics when bounding dimensionality is inconsistent.
pub fn sibling_overlap_extent_sum(left_bounds: &BoundingBox, right_bounds: &BoundingBox) -> Scalar {
    assert_eq!(
        left_bounds.dimensions(),
        right_bounds.dimensions(),
        "sibling bounds must have matching dimensionality"
    );

    bounds_overlap_extent_sum_prevalidated(left_bounds, right_bounds)
}

/// Computes summed overlap extent for already-compatible bounding boxes.
///
/// # Runtime Role
///
/// This is the shared implementation for sibling-overlap diagnostics and split
/// scoring. Callers own the release-mode dimensionality contract so hot split
/// scoring can keep its existing debug-only validation behavior.
pub(crate) fn bounds_overlap_extent_sum_prevalidated(
    left_bounds: &BoundingBox,
    right_bounds: &BoundingBox,
) -> Scalar {
    debug_assert_eq!(
        left_bounds.dimensions(),
        right_bounds.dimensions(),
        "bounds should have matching dimensionality"
    );

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
