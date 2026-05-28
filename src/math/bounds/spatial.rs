//! Bounding box spatial predicates.

use crate::math::Vector;

use super::BoundingBox;

impl BoundingBox {
    /// Returns the number of dimensions represented by the bounding box.
    pub fn dimensions(&self) -> usize {
        self.min.len()
    }

    /// Returns true when the point lies inside the bounding box.
    ///
    /// Boundary values are treated as contained.
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

    /// Returns true when two bounding boxes intersect.
    ///
    /// # Runtime Role
    ///
    /// This is the core geometric pruning test used during metadata traversal.
    ///
    /// # Formal Reference
    ///
    /// This implements the condition $Q \cap B_k \neq \emptyset$.
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

    /// Returns true when this bounding box fully contains another bounding box.
    ///
    /// # Runtime Role
    ///
    /// Parent-child containment validation uses this method to ensure hierarchy
    /// bounds remain structurally valid.
    ///
    /// # Formal Reference
    ///
    /// This corresponds to the recursive containment requirement that descendant
    /// bounding regions remain contained within ancestor bounding regions.
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
