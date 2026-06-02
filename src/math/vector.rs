//! Coordinate vector representation.

use std::error::Error;
use std::fmt;

/// Scalar coordinate type used throughout the FSE runtime.
///
/// `f32` is used for the initial implementation to match the planned SIMD path.
/// Precision-sensitive experiments can later introduce a configurable scalar type.
pub type Scalar = f32;

/// Error returned when checked coordinate vector construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VectorError {
    /// No coordinates were provided.
    Empty,
    /// A coordinate was not finite.
    NonFinite {
        /// Dimension containing the non-finite coordinate.
        dimension: usize,
    },
}

impl fmt::Display for VectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => {
                formatter.write_str("coordinate vector must have at least one dimension")
            }
            Self::NonFinite { .. } => {
                formatter.write_str("coordinate vector values must be finite")
            }
        }
    }
}

impl Error for VectorError {}

/// A point in the ambient coordinate space.
///
/// # Runtime Role
///
/// `Vector` represents a record coordinate in the embedded space used by FSE.
///
/// # Formal Reference
///
/// This structure corresponds to a point `x` in the dataset `D`.
#[derive(Clone, Debug, PartialEq)]
pub struct Vector {
    /// Coordinate values for the point.
    pub values: Vec<Scalar>,
}

impl Vector {
    /// Creates a new vector from a sequence of coordinate values.
    ///
    /// # Panics
    ///
    /// Panics when no coordinates are provided or when any coordinate is not
    /// finite.
    pub fn new(values: Vec<Scalar>) -> Vector {
        Self::try_new(values).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a new vector and returns an error when coordinates are invalid.
    ///
    /// # Runtime Role
    ///
    /// This constructor is intended for caller-facing input validation. It
    /// enforces the same invariants as [`Vector::new`] without panicking.
    pub fn try_new(values: Vec<Scalar>) -> Result<Vector, VectorError> {
        if values.is_empty() {
            return Err(VectorError::Empty);
        }

        for (dimension, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(VectorError::NonFinite { dimension });
            }
        }

        Ok(Vector { values })
    }

    /// Returns the number of dimensions represented by the vector.
    pub fn dimensions(&self) -> usize {
        self.values.len()
    }

    /// Returns true when the vector contains no coordinates.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
