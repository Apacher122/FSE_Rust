//! Deterministic ordering utilities for benchmark result comparison.

use std::cmp::Ordering;

use crate::math::{Scalar, Vector};

/// Sorts points lexicographically for order-independent benchmark comparison.
///
/// # Runtime Role
///
/// Query engines are allowed to return matching records in different traversal
/// orders. This helper normalizes result ordering before equality checks so
/// benchmark comparisons verify set equality rather than traversal order.
///
/// # Ordering Rule
///
/// Points are ordered by coordinate values from left to right. If all shared
/// coordinates are equal, shorter vectors sort before longer vectors.
pub fn sort_points_lexicographically(points: &mut [Vector]) {
    points.sort_by(compare_points_lexicographically);
}

/// Compares two points using the benchmark result ordering rule.
///
/// # Runtime Role
///
/// This comparator is shared by production benchmark comparison code and test
/// support so correctness checks use one consistent ordering definition.
pub fn compare_points_lexicographically(left: &Vector, right: &Vector) -> Ordering {
    for (left_value, right_value) in left.values.iter().zip(&right.values) {
        match compare_scalar_values(left_value, right_value) {
            Ordering::Equal => continue,
            ordering => return ordering,
        }
    }

    left.values.len().cmp(&right.values.len())
}

fn compare_scalar_values(left: &Scalar, right: &Scalar) -> Ordering {
    left.partial_cmp(right).unwrap_or(Ordering::Equal)
}
