//! Partition node representation.

use crate::math::{BoundingBox, ResidualBlock, Scalar, Vector, compute_centroid};

/// A structural partition within the Fractal Semantic Encoding (FSE) runtime.
///
/// `PartitionNode` serves as the core unit of the hierarchy, storing local metadata and
/// the residual representation for a specific spatial region. A node can represent
/// either an internal routing element or a terminal leaf. For leaf nodes, the
/// residuals store the compressed records used for reconstruction during query
/// execution. Internal nodes typically leave residuals empty but track the total
/// `cardinality` of the underlying subtree. Formally, this structure corresponds
/// to the partition tuple $P_k = (D_k, \mu_k, B_k, \Delta_k)$.
#[derive(Clone, Debug, PartialEq)]
pub struct PartitionNode {
    /// A stable, unique identifier for the node within the index.
    pub id: usize,
    /// The geometric center of the partition.
    pub centroid: Vec<Scalar>,
    /// The axis-aligned bounded support region for the partition.
    pub bounds: BoundingBox,
    /// The centroid-relative residual representation of the contained data.
    pub residuals: ResidualBlock,
    /// The total number of records represented by this node or its entire subtree.
    pub cardinality: usize,

    // Vec<usize> means a heap allocation per node for children.
    // Keeping it as a Vec for V1 flexibility.
    pub children: Vec<usize>,

    /// A flag indicating whether this node is a terminal leaf partition.
    pub is_leaf: bool,
}

impl PartitionNode {
    /// Creates a new partition node from explicit components, deriving the
    /// record count directly from the residual block.
    ///
    /// # Panics
    ///
    /// Panics if the dimensionality of the centroid, bounds, or residual block
    /// is inconsistent.
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

    /// Constructs a partition node with an explicitly defined record count.
    ///
    /// This constructor is primarily used for internal hierarchy nodes that summarize
    /// a large subtree without physically storing residual rows for reconstruction.
    ///
    /// # Panics
    ///
    /// Panics if the dimensionality of the centroid, bounds, or residual block
    /// is inconsistent, or if the centroid is empty.
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

    /// Builds a leaf partition directly from a set of coordinate vectors.
    ///
    /// This method automatically calculates the local centroid, exact bounding region,
    /// and relative residuals for the provided point set. It is particularly useful
    /// for establishing the terminal layers of the index.
    ///
    /// # Panics
    ///
    /// Panics if the point set is empty or if coordinate dimensionalities are inconsistent.
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

    /// Constructs an internal partition node that summarizes a set of points and
    /// manages child identifiers.
    ///
    /// Unlike leaf partitions, internal nodes prioritize metadata for traversal and
    /// total subtree cardinality over local residual storage.
    ///
    /// # Panics
    ///
    /// Panics if the point set is empty or if coordinate dimensionalities are inconsistent.
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

    /// Returns the dimensionality of the partition's coordinate space.
    pub fn dimensions(&self) -> usize {
        self.centroid.len()
    }

    /// Checks whether the partition contains references to child nodes.
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Returns the number of residual coordinate rows physically stored on this node.
    pub fn stored_cardinality(&self) -> usize {
        self.residuals.cardinality()
    }
}
