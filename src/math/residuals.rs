//! Centroid-relative residual storage.

use crate::math::{Scalar, Vector};

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualBlock {
    pub dimensions: Vec<Vec<Scalar>>,
}

impl ResidualBlock {
    pub fn new(dimensions: Vec<Vec<Scalar>>) -> Self {
        Self { dimensions }
    }

    pub fn from_points(points: &[Vector], centroid: &[Scalar]) -> Self {
        let dimensions = centroid.len();
        let mut residuals = vec![Vec::with_capacity(points.len()); dimensions];

        for point in points {
            for dimension in 0..dimensions {
                residuals[dimension].push(point.values[dimension] - centroid[dimension]);
            }
        }
        Self {
            dimensions: residuals,
        }
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions.len()
    }
    pub fn cardinality(&self) -> usize {
        self.dimensions.first().map_or(0, Vec::len)
    }
    pub fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }
}
