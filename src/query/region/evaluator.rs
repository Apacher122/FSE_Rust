//! Exact point-level query region evaluation.

use crate::math::Vector;

use super::QueryRegion;

/// Evaluates reconstructed points against a query region.
///
/// # Runtime Role
///
/// Region evaluation filters reconstructed candidate records after metadata
/// pruning and deferred reconstruction have already occurred.
///
/// # Formal Reference
///
/// This implements the logical evaluation function `q(x)`, where a point is
/// retained exactly when `x` lies inside the query region `Q`.
pub fn evaluate_query(points: &[Vector], query: &QueryRegion) -> Vec<Vector> {
    let mut matches = Vec::new();

    for point in points {
        if query.contains_point(point) {
            matches.push(point.clone());
        }
    }

    matches
}
