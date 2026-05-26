//! Portable terminal duration formatting.

use std::time::Duration;

/// Formats a duration with ASCII-only unit suffixes for terminal artifacts.
///
/// Rust's debug formatting uses a microsecond suffix that may be misread by
/// some shell capture paths. Benchmark artifacts use this formatter so uploaded
/// text reports remain portable across terminals and editors.
pub fn format_duration_ascii(duration: Duration) -> String {
    let nanos = duration.as_nanos();

    if nanos < 1_000 {
        return format!("{}ns", nanos);
    }

    if nanos < 1_000_000 {
        return format_scaled_duration(nanos, 1_000, 3, "us");
    }

    if nanos < 1_000_000_000 {
        return format_scaled_duration(nanos, 1_000_000, 6, "ms");
    }

    format_scaled_duration(nanos, 1_000_000_000, 9, "s")
}

fn format_scaled_duration(
    nanos: u128,
    unit_nanos: u128,
    fractional_width: usize,
    suffix: &str,
) -> String {
    let whole = nanos / unit_nanos;
    let remainder = nanos % unit_nanos;

    if remainder == 0 {
        return format!("{}{}", whole, suffix);
    }

    let mut fractional = format!("{:0width$}", remainder, width = fractional_width);

    while fractional.ends_with('0') {
        fractional.pop();
    }

    format!("{}.{}{}", whole, fractional, suffix)
}
