//! Split acceptance policy.

use crate::build::metrics::SplitQualityMetrics;

/// Returns whether a split improves structural geometry.
///
/// # Runtime Role
///
/// The builder uses this as the optional split acceptance criterion. A split is
/// useful when it reduces combined child volume. If the parent volume is zero,
/// volume cannot be reduced, so extent reduction becomes the fallback signal.
pub fn accepts_split_quality(metrics: &SplitQualityMetrics) -> bool {
    if metrics.reduces_volume() {
        return true;
    }

    // skinny data still deserves a useful split
    metrics.parent_volume == 0.0 && metrics.reduces_extent()
}
