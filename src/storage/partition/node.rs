//! Partition node data shape.

use crate::math::{BoundingBox, ResidualBlock, Scalar};

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
