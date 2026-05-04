//! Centroid calculation utilities.
use crate::math::{Scalar, Vector};

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
