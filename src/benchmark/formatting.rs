//! Shared benchmark formatting helpers.

use crate::math::Scalar;

/// Formats a scalar with two fractional digits for compact terminal output.
pub(crate) fn format_scalar_fixed_2(value: Scalar) -> String {
    format!("{:.2}", value)
}

/// Formats an `f64` with two fractional digits for compact terminal output.
pub(crate) fn format_f64_fixed_2(value: f64) -> String {
    format!("{:.2}", value)
}

/// Formats a scalar with six fractional digits for stable report output.
pub(crate) fn format_scalar_fixed_6(value: Scalar) -> String {
    format!("{:.6}", value)
}

/// Formats an `f64` with six fractional digits for stable report output.
pub(crate) fn format_f64_fixed_6(value: f64) -> String {
    format!("{:.6}", value)
}
