//! Encoded coordinate values.

use crate::math::Scalar;

/// Numeric coordinate output produced by an encoder.
///
/// # Runtime Role
///
/// Encoders map typed logical values into one or more numeric coordinates.
/// These coordinates are later used by the FSE geometry layer.
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedCoordinates {
    values: Vec<Scalar>,
}

impl EncodedCoordinates {
    /// Creates encoded coordinates from numeric values.
    pub fn new(values: Vec<Scalar>) -> Self {
        Self { values }
    }

    /// Returns encoded coordinate values.
    pub fn values(&self) -> &[Scalar] {
        &self.values
    }

    /// Returns the number of encoded coordinate dimensions.
    pub fn dimensions(&self) -> usize {
        self.values.len()
    }

    /// Returns true when no coordinates were encoded.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
