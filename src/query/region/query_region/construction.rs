//! Query region construction and shape helpers.

use std::error::Error;
use std::fmt;

use crate::math::{BoundingBox, Scalar};

use super::QueryRegion;

/// Error returned when checked query region construction fails.
///
/// # Runtime Role
///
/// `QueryRegionError` lets caller-facing code validate query bounds without
/// relying on panic-based construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryRegionError {
    /// Minimum and maximum coordinate vectors do not have the same length.
    DimensionMismatch {
        /// Number of minimum-bound dimensions.
        min_dimensions: usize,

        /// Number of maximum-bound dimensions.
        max_dimensions: usize,
    },

    /// No query dimensions were provided.
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

impl fmt::Display for QueryRegionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { .. } => {
                formatter.write_str("query min and max vectors must have the same dimensionality")
            }
            Self::Empty => formatter.write_str("query region must have at least one dimension"),
            Self::NonFinite { .. } => {
                formatter.write_str("query bounds must be finite in every dimension")
            }
            Self::InvertedRange { dimension } => {
                write!(
                    formatter,
                    "query minimum must not exceed maximum in dimension {dimension}"
                )
            }
        }
    }
}

impl Error for QueryRegionError {}

impl QueryRegion {
    /// Creates a query region from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics when minimum and maximum vectors have different dimensionality,
    /// when no dimensions are provided, when any bound is not finite, or when
    /// any dimension has a minimum greater than its maximum.
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        Self::try_new(min, max).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a query region and returns an error when bounds are invalid.
    ///
    /// # Runtime Role
    ///
    /// This constructor is intended for caller-facing input validation. It
    /// enforces the same invariants as [`QueryRegion::new`] without panicking.
    pub fn try_new(min: Vec<Scalar>, max: Vec<Scalar>) -> Result<Self, QueryRegionError> {
        if min.len() != max.len() {
            return Err(QueryRegionError::DimensionMismatch {
                min_dimensions: min.len(),
                max_dimensions: max.len(),
            });
        }

        if min.is_empty() {
            return Err(QueryRegionError::Empty);
        }

        for (dimension, (minimum, maximum)) in min.iter().zip(&max).enumerate() {
            if !minimum.is_finite() || !maximum.is_finite() {
                return Err(QueryRegionError::NonFinite { dimension });
            }

            if minimum > maximum {
                return Err(QueryRegionError::InvertedRange { dimension });
            }
        }

        Ok(Self { min, max })
    }

    /// Returns the number of dimensions represented by the query region.
    pub fn dimensions(&self) -> usize {
        self.min.len()
    }

    /// Converts the query region into a bounding box.
    ///
    /// # Runtime Role
    ///
    /// This allows query-box intersection to reuse the same bounded-region logic
    /// used by partition metadata.
    ///
    /// # Notes
    ///
    /// This allocates a new bounding box. Hot traversal code should prefer
    /// [`QueryRegion::classify_bounds`] when it needs traversal classification
    /// and [`QueryRegion::intersects_bounds`] when it only needs intersection.
    pub fn as_bounds(&self) -> BoundingBox {
        BoundingBox::new(self.min.clone(), self.max.clone())
    }
}
