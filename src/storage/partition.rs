//! Partition node representation.

use crate::math::{BoundingBox, ResidualBlock, Scalar, Vector, compute_centroid};

#[derive(Clone, Debug, PartialEq)]
pub struct PartitionNode {
    pub id: usize,
    pub centroid: Vec<Scalar>,
    pub bounds: BoundingBox,
    pub residuals: ResidualBlock,
    pub cardinality: usize,

    // Vec<usize> means a heap allocation per node for children.
    // Keeping it as a Vec for V1 flexibility.
    pub children: Vec<usize>,

    pub is_leaf: bool,
}

impl PartitionNode {
    pub fn new(
        id: usize,
        centroid: Vec<Scalar>,
        bounds: BoundingBox,
        residuals: ResidualBlock,
        children: Vec<usize>,
        is_leaf: bool,
    ) -> Self {
        let cardinality = residuals.cardinality();
        Self::with_cardinality(
            id,
            centroid,
            bounds,
            residuals,
            cardinality,
            children,
            is_leaf,
        )
    }

    pub fn with_cardinality(
        id: usize,
        centroid: Vec<Scalar>,
        bounds: BoundingBox,
        residuals: ResidualBlock,
        cardinality: usize,
        children: Vec<usize>,
        is_leaf: bool,
    ) -> Self {
        let dimensions = centroid.len();
        assert!(dimensions > 0, "partition centroid must not be empty");
        assert_eq!(
            bounds.dimensions(),
            dimensions,
            "partition bounds must match centroid dimensionality"
        );
        assert_eq!(
            residuals.dimensions(),
            dimensions,
            "partition residuals must match centroid dimensionality"
        );

        Self {
            id,
            centroid,
            bounds,
            residuals,
            cardinality,
            children,
            is_leaf,
        }
    }

    pub fn from_points(id: usize, points: &[Vector]) -> Self {
        assert!(
            !points.is_empty(),
            "cannot build a partition from an empty point set"
        );
        let centroid = compute_centroid(points);
        let bounds = BoundingBox::from_points(points);
        let residuals = ResidualBlock::from_points(points, &centroid);
        Self::new(id, centroid, bounds, residuals, Vec::new(), true)
    }

    pub fn internal_from_points(id: usize, points: &[Vector], children: Vec<usize>) -> Self {
        assert!(
            !points.is_empty(),
            "cannot build an internal partition from an empty point set"
        );
        let centroid = compute_centroid(points);
        let bounds = BoundingBox::from_points(points);
        let residuals = ResidualBlock::new(vec![Vec::new(); centroid.len()]);
        Self::with_cardinality(
            id,
            centroid,
            bounds,
            residuals,
            points.len(),
            children,
            false,
        )
    }

    pub fn dimensions(&self) -> usize {
        self.centroid.len()
    }
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
    pub fn stored_cardinality(&self) -> usize {
        self.residuals.cardinality()
    }
}
