//! Axis-aligned bounding regions.

use crate::math::{Scalar, Vector};

/// Axis-aligned bounding box used for partition-level pruning.
///
/// # Runtime Role
///
/// `BoundingBox` stores the minimum and maximum coordinate value for each
/// dimension of a partition.
///
/// # Formal Reference
///
/// This structure corresponds to the bounded support region `B_k`.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundingBox {
    /// Minimum coordinate value per dimension.
    pub min: Vec<Scalar>,

    /// Maximum coordinate value per dimension.
    pub max: Vec<Scalar>,
}

impl BoundingBox {
    /// Creates a bounding box from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics when the minimum and maximum vectors have different dimensions.
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        assert_eq!(
            min.len(),
            max.len(),
            "bounding box min and max vectors must have the same dimensionality"
        );
        Self { min, max }
    }

    /// Builds the exact bounding box for a non-empty set of points.
    ///
    /// # Runtime Role
    ///
    /// Computes the smallest axis-aligned box containing every provided point.
    ///
    /// # Formal Reference
    ///
    /// This implements the extrema construction for `B_k`.
    ///
    /// # Panics
    ///
    /// Panics when no points are provided or when dimensionality is inconsistent.
    pub fn from_points(points: &[Vector]) -> Self {
        assert!(
            !points.is_empty(),
            "cannot construct a bounding box from an empty point set"
        );

        let dimensions = points[0].dimensions();
        assert!(dimensions > 0, "points must have at least one dimension");
        let mut min = vec![Scalar::INFINITY; dimensions];
        let mut max = vec![Scalar::NEG_INFINITY; dimensions];

        for point in points {
            assert_eq!(
                point.dimensions(),
                dimensions,
                "all points must have the same dimensionality"
            );
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
    /// This implements the condition `Q intersect B_k != empty`.
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

    /// Returns the volume of the bounding box.
    ///
    /// # Runtime Role
    ///
    /// Volume is used by structural density metrics to estimate how tightly a
    /// partition's bounded support represents its contained records.
    ///
    /// # Formal Reference
    ///
    /// This corresponds to `Vol(B_k)`.
    ///
    /// # Notes
    ///
    /// Degenerate dimensions with zero width produce zero volume.
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
