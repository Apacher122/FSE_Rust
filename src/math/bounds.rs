//! Axis-aligned bounding regions.

use crate::math::{Scalar, Vector};

/// An axis-aligned bounding box used to spatially prune partitions during queries.
///
/// A `BoundingBox` defines a rectangular region in multidimensional space by tracking
/// the minimum and maximum coordinate values along each dimension. In the formal FSE
/// specification, this corresponds to the bounded support region $B_k$.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundingBox {
    /// The minimum coordinate value for each dimension.
    pub min: Vec<Scalar>,
    /// The maximum coordinate value for each dimension.
    pub max: Vec<Scalar>,
}

impl BoundingBox {
    /// Creates a new bounding box from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics if the `min` and `max` vectors have different dimensionalities.
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        assert_eq!(
            min.len(),
            max.len(),
            "bounding box min and max vectors must have the same dimensionality"
        );
        Self { min, max }
    }

    /// Computes the smallest axis-aligned bounding box that encapsulates all provided points.
    ///
    /// This method iteratively finds the extreme minimum and maximum coordinates across
    /// all dimensions to construct the bounding region (formally known as the extrema
    /// construction for $B_k$).
    ///
    /// # Panics
    ///
    /// Panics if the provided slice of points is empty, or if any points have
    /// inconsistent dimensionalities.
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

    /// Returns the number of dimensions this bounding box spans.
    pub fn dimensions(&self) -> usize {
        self.min.len()
    }

    /// Checks if a point lies entirely within the bounding box.
    ///
    /// Points that lie exactly on the boundary are considered contained.
    /// Returns `false` if the point's dimensionality does not match the bounding box.
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

    /// Checks if this bounding box intersects with another bounding box.
    ///
    /// This is the core geometric pruning test used during metadata traversal to quickly
    /// discard partitions that do not overlap with a query region. Formally, this evaluates
    /// the condition $Q \cap B_k \neq \emptyset$.
    ///
    /// Returns `false` if the dimensionalities differ.
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

    /// Calculates the multidimensional volume of the bounding box.
    ///
    /// Volume is used as a structural density metric to estimate how tightly a partition's
    /// bounding region encapsulates its underlying records. Formally, this corresponds to $\text{Vol}(B_k)$.
    ///
    /// # Notes
    ///
    /// * If any dimension has a width of exactly zero (a degenerate dimension), the
    ///   resulting volume will be `0.0`.
    /// * Returns `0.0` if any minimum coordinate exceeds its corresponding maximum.
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

    /// Checks if this bounding box fully encloses another bounding box.
    ///
    /// This is primarily used during parent-child containment validation to ensure that the
    /// hierarchy bounds remain structurally valid. It verifies the recursive requirement
    /// that all descendant bounding regions fit entirely within their ancestor's region.
    ///
    /// Returns `false` if the dimensionalities differ.
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
