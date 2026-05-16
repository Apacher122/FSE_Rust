//! Partition node representation.

use crate::math::{BoundingBox, ResidualBlock, Scalar, Vector, compute_centroid};

/// A structural partition in the FSE runtime.
///
/// # Runtime Role
///
/// `PartitionNode` stores the local metadata and residual representation for a
/// partition. It may represent either an internal hierarchy node or a leaf node.
///
/// For leaf nodes, residuals store the records reconstructed during query
/// execution. For internal nodes, residuals may be empty while `cardinality`
/// still records the total number of records represented by the subtree.
///
/// # Formal Reference
///
/// This structure corresponds to the partition tuple `P_k = (D_k, mu_k, B_k, Delta_k)`.
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

    // vec is fine for now because split fanout may change later
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
    /// is inconsistent, if the centroid is empty, if a leaf node has a cardinality
    /// different from its stored residual row count, or if stored residual rows
    /// exceed the declared subtree cardinality.
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

        let stored_cardinality = residuals.cardinality();

        if is_leaf {
            // leaves are the only nodes that must physically hold every row
            assert_eq!(
                stored_cardinality, cardinality,
                "leaf partition cardinality must match stored residual row count"
            );
        } else {
            // internal nodes can count records without storing them here
            assert!(
                stored_cardinality <= cardinality,
                "stored residual rows must not exceed partition cardinality"
            );
        }

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
