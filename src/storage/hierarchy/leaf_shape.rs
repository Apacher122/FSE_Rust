//! Cached leaf reconstruction metadata.

/// Cached reconstruction shape for a leaf partition.
///
/// # Runtime Role
///
/// `LeafReconstructionShape` stores the small amount of immutable metadata
/// needed by retained-leaf execution. Query execution can use this value instead
/// of revalidating leaf residual shape every time a leaf is reconstructed.
///
/// # Notes
///
/// The `node_id` is the position of the leaf in `FSEIndex::nodes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafReconstructionShape {
    /// Leaf node identifier in the index node list.
    pub node_id: usize,

    /// Number of coordinate dimensions reconstructed for each row.
    pub dimensions: usize,

    /// Number of residual rows stored by the leaf.
    pub cardinality: usize,
}

impl LeafReconstructionShape {
    /// Creates cached reconstruction metadata for a leaf node.
    pub fn new(node_id: usize, dimensions: usize, cardinality: usize) -> Self {
        Self {
            node_id,
            dimensions,
            cardinality,
        }
    }
}
