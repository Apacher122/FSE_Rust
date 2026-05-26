//! Formatting helpers for target workload diagnostics.

use crate::benchmark::formatting::format_scalar_fixed_2;
use crate::math::{BoundingBox, Scalar};
use crate::query::RetainedLeafCoverage;

pub(super) fn format_percent_ratio(value: f64) -> String {
    if value.is_infinite() {
        return "inf".to_string();
    }

    format!("{:.2}%", value * 100.0)
}

pub(super) fn format_speedup_ratio(value: f64) -> String {
    if value.is_infinite() {
        return "inf".to_string();
    }

    format!("{:.2}x", value)
}

pub(super) fn retained_leaf_coverage_label(coverage: RetainedLeafCoverage) -> &'static str {
    match coverage {
        RetainedLeafCoverage::Covered => "covered",
        RetainedLeafCoverage::Partial => "partial",
    }
}

pub(super) fn format_bounds_min(bounds: &BoundingBox) -> String {
    format_coordinate_values(&bounds.min)
}

pub(super) fn format_bounds_max(bounds: &BoundingBox) -> String {
    format_coordinate_values(&bounds.max)
}

pub(super) fn format_coordinate_values(values: &[Scalar]) -> String {
    let formatted_values: Vec<String> = values
        .iter()
        .map(|value| format_scalar_fixed_2(*value))
        .collect();

    format!("[{}]", formatted_values.join(", "))
}
