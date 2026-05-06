//! Centroid calculation utilities.
use crate::math::{Scalar, Vector};

/// Calculates the geometric centroid for a given set of points.
///
/// The centroid serves as the local geometric center and acts as the reference
/// origin for residual encoding within a partition. In the formal FSE
/// specification, this corresponds to the partition centroid $\mu_k$.
///
/// # Panics
///
/// Panics if the provided slice of points is empty, or if any points have
/// inconsistent dimensionalities.
pub fn compute_centroid(points: &[Vector]) -> Vec<Scalar> {
    assert!(
        !points.is_empty(),
        "cannot compute centroid for empty points"
    );
    let dimensions = points[0].dimensions();
    let mut centroid = vec![0.0; dimensions];

    for point in points {
        for dimension in 0..dimensions {
            centroid[dimension] += point.values[dimension];
        }
    }

    let count = points.len() as Scalar;
    for value in &mut centroid {
        *value /= count;
    }
    centroid
}
