//! Residual block construction helpers.

use std::error::Error;
use std::fmt;

use crate::math::{Scalar, Vector};

use super::ResidualBlock;

/// Error returned when checked residual block construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidualBlockError {
    /// A residual dimension had a different row count than the first dimension.
    UnevenDimensionLength {
        /// Dimension containing the mismatched row count.
        dimension: usize,
        /// Row count found in the mismatched dimension.
        actual_rows: usize,
        /// Row count expected from the first dimension.
        expected_rows: usize,
    },
    /// A residual value was not finite.
    NonFinite {
        /// Dimension containing the non-finite value.
        dimension: usize,
        /// Row containing the non-finite value.
        row: usize,
    },
}

impl fmt::Display for ResidualBlockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnevenDimensionLength {
                dimension,
                actual_rows,
                expected_rows,
            } => write!(
                formatter,
                "residual dimension {dimension} has {actual_rows} rows but expected {expected_rows}"
            ),
            Self::NonFinite { .. } => formatter.write_str("residual values must be finite"),
        }
    }
}

impl Error for ResidualBlockError {}

impl ResidualBlock {
    /// Creates a new residual block from residual values.
    ///
    /// # Runtime Role
    ///
    /// This constructor validates that every residual dimension contains the
    /// same number of rows. That invariant is required by reconstruction because
    /// each logical record is reconstructed by reading one value from every
    /// dimension at the same row index.
    ///
    /// # Panics
    ///
    /// Panics when residual dimensions do not contain the same number of rows
    /// or when any residual value is not finite.
    pub fn new(dimensions: Vec<Vec<Scalar>>) -> Self {
        Self::try_new(dimensions).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a residual block and returns an error when residuals are invalid.
    ///
    /// # Runtime Role
    ///
    /// This constructor is intended for caller-facing input validation. It
    /// enforces the same invariants as [`ResidualBlock::new`] without panicking.
    pub fn try_new(dimensions: Vec<Vec<Scalar>>) -> Result<Self, ResidualBlockError> {
        Self::validate_consistent_shape(&dimensions)?;
        Self::validate_finite_values(&dimensions)?;

        Ok(Self { dimensions })
    }

    /// Builds a residual block from points and a centroid.
    ///
    /// # Runtime Role
    ///
    /// Converts absolute coordinates into centroid-relative residual values.
    ///
    /// # Formal Reference
    ///
    /// This implements $\Delta_k(x) = x - \mu_k$.
    ///
    /// # Panics
    ///
    /// Panics when dimensionality is inconsistent or when a centroid or point
    /// coordinate is not finite.
    pub fn from_points(points: &[Vector], centroid: &[Scalar]) -> Self {
        let dimensions = centroid.len();

        assert!(dimensions > 0, "centroid must have at least one dimension");
        assert!(
            centroid.iter().all(|value| value.is_finite()),
            "centroid values must be finite"
        );

        let mut residuals = vec![Vec::with_capacity(points.len()); dimensions];

        for point in points {
            assert_eq!(
                point.dimensions(),
                dimensions,
                "point and centroid dimensionality must match"
            );

            for dimension in 0..dimensions {
                let coordinate = point.values[dimension];

                assert!(coordinate.is_finite(), "point coordinates must be finite");

                residuals[dimension].push(coordinate - centroid[dimension]);
            }
        }

        Self::new(residuals)
    }

    fn validate_consistent_shape(dimensions: &[Vec<Scalar>]) -> Result<(), ResidualBlockError> {
        let Some(first_dimension) = dimensions.first() else {
            return Ok(());
        };

        let expected_rows = first_dimension.len();

        // dont let one dimension quietly drift away from the others
        for (dimension_index, dimension) in dimensions.iter().enumerate().skip(1) {
            if dimension.len() != expected_rows {
                return Err(ResidualBlockError::UnevenDimensionLength {
                    dimension: dimension_index,
                    actual_rows: dimension.len(),
                    expected_rows,
                });
            }
        }

        Ok(())
    }

    fn validate_finite_values(dimensions: &[Vec<Scalar>]) -> Result<(), ResidualBlockError> {
        for (dimension, values) in dimensions.iter().enumerate() {
            for (row, value) in values.iter().enumerate() {
                if !value.is_finite() {
                    return Err(ResidualBlockError::NonFinite { dimension, row });
                }
            }
        }

        Ok(())
    }
}
