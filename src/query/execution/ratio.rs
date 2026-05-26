//! Shared query execution ratio helpers.

use crate::math::Scalar;

/// Divides two counts as a scalar ratio, returning zero for empty denominators.
///
/// # Runtime Role
///
/// Query execution reports several work ratios. Empty indexes and empty leaf
/// sets should report zero instead of producing a divide-by-zero result.
pub(crate) fn ratio_or_zero(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        0.0
    } else {
        numerator as Scalar / denominator as Scalar
    }
}
