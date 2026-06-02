//! Bounding box construction helpers.

use std::error::Error;
use std::fmt;

use crate::math::{Scalar, Vector};

use super::BoundingBox;

/// Error returned when checked bounding box construction fails.
///
/// # Runtime Role
///
/// `BoundingBoxError` lets caller-facing code validate bounded-support regions
/// without relying on panic-based construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundingBoxError {
    /// Minimum and maximum coordinate vectors do not have the same length.
    DimensionMismatch {
        /// Number of minimum-bound dimensions.
        min_dimensions: usize,

        /// Number of maximum-bound dimensions.
        max_dimensions: usize,
    },

    /// No dimensions were provided.
    Empty,

    /// At least one bound value is not finite.
    NonFinite {
        /// Dimension containing the non-finite value.
        dimension: usize,
    },

    /// A minimum bound is greater than its maximum bound.
    InvertedRange {
        /// Dimension containing the inverted range.
        dimension: usize,
    },
}

impl fmt::Display for BoundingBoxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { .. } => formatter
                .write_str("bounding box min and max vectors must have the same dimensionality"),
            Self::Empty => formatter.write_str("bounding box must have at least one dimension"),
            Self::NonFinite { .. } => {
                formatter.write_str("bounding box bounds must be finite in every dimension")
            }
            Self::InvertedRange { dimension } => {
                write!(
                    formatter,
                    "bounding box minimum must not exceed maximum in dimension {dimension}"
                )
            }
        }
    }
}

impl Error for BoundingBoxError {}

impl BoundingBox {
    /// Creates a bounding box from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics when the minimum and maximum vectors have different dimensions,
    /// when no dimensions are provided, when any bound is not finite, or when
    /// any dimension has a minimum greater than its maximum.
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        Self::try_new(min, max).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a bounding box and returns an error when bounds are invalid.
    ///
    /// # Runtime Role
    ///
    /// This constructor is intended for caller-facing input validation. It
    /// enforces the same invariants as [`BoundingBox::new`] without panicking.
    pub fn try_new(min: Vec<Scalar>, max: Vec<Scalar>) -> Result<Self, BoundingBoxError> {
        if min.len() != max.len() {
            return Err(BoundingBoxError::DimensionMismatch {
                min_dimensions: min.len(),
                max_dimensions: max.len(),
            });
        }

        if min.is_empty() {
            return Err(BoundingBoxError::Empty);
        }

        validate_explicit_bounds(&min, &max)?;

        Ok(Self { min, max })
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

fn validate_explicit_bounds(min: &[Scalar], max: &[Scalar]) -> Result<(), BoundingBoxError> {
    for (dimension, (minimum, maximum)) in min.iter().zip(max).enumerate() {
        if !minimum.is_finite() || !maximum.is_finite() {
            return Err(BoundingBoxError::NonFinite { dimension });
        }

        if minimum > maximum {
            return Err(BoundingBoxError::InvertedRange { dimension });
        }
    }

    Ok(())
}
