//! Structural metrics for partition quality.
//!
//! This module contains structural metrics used to inspect FSE build quality.
//! Split-quality metrics, index density metrics, and sibling-overlap metrics are
//! split by responsibility while preserving the existing public API.

mod density;
mod overlap;
mod split_quality;
mod types;

pub use density::{index_density, index_structure_metrics, partition_density};
pub(crate) use overlap::bounds_overlap_extent_sum_prevalidated;
pub use overlap::{sibling_overlap_extent_sum, sibling_overlap_metrics};
pub use split_quality::{
    bounding_extent_sum, split_quality_metrics, split_quality_metrics_for_axis,
    split_quality_metrics_from_bounds,
};
pub use types::{IndexStructureMetrics, SiblingOverlapMetrics, SplitQualityMetrics};
