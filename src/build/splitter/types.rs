//! Splitter result and score types.

use crate::build::metrics::SplitQualityMetrics;
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
