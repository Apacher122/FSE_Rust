//! Coordinate vector representation.

/// Scalar coordinate type used throughout the FSE runtime.
///
/// `f32` is used for the initial implementation to match the planned SIMD path.
/// Precision-sensitive experiments can later introduce a configurable scalar type.
pub type Scalar = f32;

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
    pub fn new(values: Vec<Scalar>) -> Vector {
        Vector { values }
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
