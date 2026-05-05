//! Query region representation.

use crate::math::{BoundingBox, Scalar, Vector};

#[derive(Clone, Debug, PartialEq)]
pub struct QueryRegion {
    pub min: Vec<Scalar>,
    pub max: Vec<Scalar>,
}

impl QueryRegion {
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        assert_eq!(
            min.len(),
            max.len(),
            "query min and max vectors must have the same dimensionality"
        );
        assert!(
            !min.is_empty(),
            "query region must have at least one dimension"
        );
        Self { min, max }
    }

    pub fn dimensions(&self) -> usize {
        self.min.len()
    }

    pub fn as_bounds(&self) -> BoundingBox {
        BoundingBox::new(self.min.clone(), self.max.clone())
    }

    pub fn contains_point(&self, point: &Vector) -> bool {
        if point.dimensions() != self.dimensions() {
            return false;
        }
        for dimension in 0..self.dimensions() {
            let value = point.values[dimension];
            if value < self.min[dimension] || value > self.max[dimension] {
                return false;
            }
        }
        true
    }
}
