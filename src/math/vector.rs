//! Coordinate vector representation.

pub type Scalar = f32; // Make 'Scalar' generic <T> if f64 is needed.

/// A point in the ambient coordinate space.
#[derive(Clone, Debug, PartialEq)]
pub struct Vector {
    pub values: Vec<Scalar>,
}

impl Vector {
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
