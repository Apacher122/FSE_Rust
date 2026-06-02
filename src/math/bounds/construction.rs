//! Bounding box construction helpers.

use crate::math::{Scalar, Vector};

use super::BoundingBox;

impl BoundingBox {
    /// Creates a bounding box from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics when the minimum and maximum vectors have different dimensions,
    /// when no dimensions are provided, when any bound is not finite, or when
    /// any dimension has a minimum greater than its maximum.
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        assert_eq!(
            min.len(),
            max.len(),
            "bounding box min and max vectors must have the same dimensionality"
        );
        assert!(
            !min.is_empty(),
            "bounding box must have at least one dimension"
        );

        validate_explicit_bounds(&min, &max);

        Self { min, max }
    }

    /// Builds the exact bounding box for a non-empty set of points.
    ///
    /// # Runtime Role
    ///
    /// Computes the smallest axis-aligned box containing every provided point.
    ///
    /// # Formal Reference
    ///
    /// This implements the extrema construction for $B_k$.
    ///
    /// # Panics
    ///
    /// Panics when no points are provided, when dimensionality is inconsistent,
    /// or when any point coordinate is not finite.
    pub fn from_points(points: &[Vector]) -> Self {
        assert!(
            !points.is_empty(),
            "cannot construct a bounding box from an empty point set"
        );

        let dimensions = points[0].dimensions();
        assert!(dimensions > 0, "points must have at least one dimension");
        let mut min = vec![Scalar::INFINITY; dimensions];
        let mut max = vec![Scalar::NEG_INFINITY; dimensions];

        for point in points {
            assert_eq!(
                point.dimensions(),
                dimensions,
                "all points must have the same dimensionality"
            );

            for dimension in 0..dimensions {
                let value = point.values[dimension];

                assert!(
                    value.is_finite(),
                    "bounding box point coordinates must be finite in every dimension"
                );

                if value < min[dimension] {
                    min[dimension] = value;
                }

                if value > max[dimension] {
                    max[dimension] = value;
                }
            }
        }

        Self::new(min, max)
    }
}

fn validate_explicit_bounds(min: &[Scalar], max: &[Scalar]) {
    for (dimension, (minimum, maximum)) in min.iter().zip(max).enumerate() {
        assert!(
            minimum.is_finite() && maximum.is_finite(),
            "bounding box bounds must be finite in every dimension"
        );
        assert!(
            minimum <= maximum,
            "bounding box minimum must not exceed maximum in dimension {dimension}"
        );
    }
}
