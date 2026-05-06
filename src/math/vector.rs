//! Coordinate vector representation.

/// The scalar coordinate type used throughout the FSE runtime.
///
/// This implementation uses `f32` to optimize for planned SIMD acceleration paths.
/// May introduce Scalar generic <T> if f64 is needed.
pub type Scalar = f32;

/// A point in the ambient coordinate space.
///
/// `Vector` represents a single record coordinate in the embedded space used
/// by the FSE architecture. In the formal specification, this structure
/// corresponds to a point $x$ within the dataset $D$.
#[derive(Clone, Debug, PartialEq)]
pub struct Vector {
    pub values: Vec<Scalar>,
}

impl Vector {
    /// Creates a new vector from a sequence of coordinate values.
    pub fn new(values: Vec<Scalar>) -> Vector {
        Vector { values }
    }

    pub fn dimensions(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
