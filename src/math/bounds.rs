//! Axis-aligned bounding regions.

use crate::math::{Scalar, Vector};

#[derive(Clone, Debug, PartialEq)]
pub struct BoundingBox {
    pub min: Vec<Scalar>,
    pub max: Vec<Scalar>,
}

impl BoundingBox {
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        assert_eq!(
            min.len(),
            max.len(),
            "bounding box min and max vectors must have the same dimensionality"
        );
        Self { min, max }
    }

    pub fn from_points(points: &[Vector]) -> Self {
        // using an assert here to fail-fast during the prototype phase if empty data is passed.
        assert!(
            !points.is_empty(),
            "cannot construct a bounding box from an empty point set"
        );

        let dimensions = points[0].dimensions();
        let mut min = vec![Scalar::INFINITY; dimensions];
        let mut max = vec![Scalar::NEG_INFINITY; dimensions];

        for point in points {
            for dimension in 0..dimensions {
                let value = point.values[dimension];
                if value < min[dimension] {
                    min[dimension] = value;
                }
                if value > max[dimension] {
                    max[dimension] = value;
                }
            }
        }
        Self { min, max }
    }

    pub fn dimensions(&self) -> usize {
        self.min.len()
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

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        if self.dimensions() != other.dimensions() {
            return false;
        }
        for dimension in 0..self.dimensions() {
            if self.max[dimension] < other.min[dimension]
                || self.min[dimension] > other.max[dimension]
            {
                return false;
            }
        }
        true
    }

    pub fn volume(&self) -> Scalar {
        let mut volume = 1.0;
        for dimension in 0..self.dimensions() {
            let width = self.max[dimension] - self.min[dimension];
            if width < 0.0 {
                return 0.0;
            }
            volume *= width;
        }
        volume
    }

    pub fn contains_bounds(&self, other: &BoundingBox) -> bool {
        if self.dimensions() != other.dimensions() {
            return false;
        }
        for dimension in 0..self.dimensions() {
            if other.min[dimension] < self.min[dimension]
                || other.max[dimension] > self.max[dimension]
            {
                return false;
            }
        }
        true
    }
}
