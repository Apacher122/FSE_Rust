//! Shared benchmark math helpers.

use std::time::Duration;

use crate::math::Scalar;

/// Divides two counts as an FSE scalar ratio, returning zero for empty denominators.
///
/// # Runtime Role
///
/// Benchmark reports use this for record-count ratios such as candidate work,
/// avoided reconstruction work, and weighted selectivity buckets.
pub(crate) fn scalar_ratio_or_zero(numerator: usize, denominator: usize) -> Scalar {
    if denominator == 0 {
        0.0
    } else {
        numerator as Scalar / denominator as Scalar
    }
}

/// Divides two counts as an `f64` ratio, returning zero for empty denominators.
///
/// # Runtime Role
///
/// Terminal benchmark rendering uses this for display-only ratios that are not
/// part of query execution's scalar report contract.
pub(crate) fn f64_ratio_or_zero(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

/// Divides a duration by a count, returning zero for empty divisors.
///
/// # Runtime Role
///
/// Benchmark timing reports and aggregate summaries both need explicit duration
/// averaging. Keeping this helper shared prevents the two reporting paths from
/// drifting.
pub(crate) fn duration_div(duration: Duration, divisor: usize) -> Duration {
    if divisor == 0 {
        return Duration::ZERO;
    }

    Duration::from_secs_f64(duration.as_secs_f64() / divisor as f64)
}
