//! Global FSE index hierarchy.

use std::error::Error;
use std::fmt;

use crate::storage::PartitionNode;

use super::LeafReconstructionShape;

/// Error returned when checked FSE index construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEIndexError {
    /// No partition nodes were provided.
    EmptyNodeList,
    /// The requested root id does not exist in the node list.
    MissingRoot {
        /// Requested root node id.
        root: usize,
        /// Number of nodes provided.
        node_count: usize,
    },
    /// A node had a different dimensionality than the root node.
    DimensionMismatch {
        /// Node containing the mismatched dimensionality.
        node: usize,
        /// Dimensionality found in the node.
        actual_dimensions: usize,
        /// Dimensionality expected from the root node.
        expected_dimensions: usize,
    },
}

impl fmt::Display for FSEIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNodeList => formatter.write_str("index must contain at least one node"),
            Self::MissingRoot { .. } => {
                formatter.write_str("root node id must exist in the node list")
            }
            Self::DimensionMismatch { .. } => {
                formatter.write_str("all nodes in an index must have the same dimensionality")
            }
        }
    }
}

impl Error for FSEIndexError {}

/// In-memory representation of an FSE hierarchy.
///
/// # Runtime Role
///
/// `FSEIndex` owns all partition nodes and identifies the root of the hierarchy.
/// Query execution traverses this structure without mutating it.
///
/// The index also caches simple structural counts, leaf identifiers, and leaf
/// reconstruction shapes that are needed during query execution. These values
/// are derived once at construction time so hot query paths do not need to
/// rescan or revalidate the whole node list for every request.
///
/// # Formal Reference
///
/// This structure corresponds to the global FSE representation
/// `F = {P_1, P_2, ..., P_K}`.
#[derive(Clone, Debug, PartialEq)]
pub struct FSEIndex {
    /// All partition nodes in the hierarchy.
    pub nodes: Vec<PartitionNode>,

    /// Node identifier of the root partition.
    pub root: usize,

    /// Dimensionality of the represented coordinate space.
    pub dimensions: usize,

    /// Number of leaf partitions in the hierarchy.
    ///
    /// # Runtime Role
    ///
    /// Traversal reports need this value for pruning statistics. Caching it here
    /// avoids a full node scan during every query.
    pub leaf_count: usize,

    /// Leaf node identifiers in node-list order.
    ///
    /// # Runtime Role
    ///
    /// Full-root coverage and parallel covered-leaf setup need to iterate all
    /// leaves. Caching these ids avoids repeatedly scanning every node and
    /// checking `is_leaf` on the hot path.
    pub leaf_node_ids: Vec<usize>,

    /// Cached reconstruction shapes for every leaf in node-list order.
    ///
    /// # Runtime Role
    ///
    /// Retained-leaf execution uses this metadata to avoid per-query leaf shape
    /// validation.
    pub leaf_reconstruction_shapes: Vec<LeafReconstructionShape>,

    /// Cached leaf reconstruction shape lookup by node id.
    ///
    /// # Runtime Role
    ///
    /// Traversal returns retained leaves by node id. This table makes the shape
    /// lookup O(1) without searching the leaf list.
    pub leaf_reconstruction_shapes_by_node: Vec<Option<LeafReconstructionShape>>,
}

impl FSEIndex {
    /// Creates a new index from partition nodes and a root identifier.
    ///
    /// # Panics
    ///
    /// Panics when the node list is empty, the root does not exist, or node
    /// dimensionality is inconsistent.
    pub fn new(nodes: Vec<PartitionNode>, root: usize) -> Self {
        Self::try_new(nodes, root).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a new index and returns an error when hierarchy metadata is invalid.
    ///
    /// # Runtime Role
    ///
    /// This constructor is intended for caller-facing input validation. It
    /// enforces the same invariants as [`FSEIndex::new`] without panicking.
    pub fn try_new(nodes: Vec<PartitionNode>, root: usize) -> Result<Self, FSEIndexError> {
        if nodes.is_empty() {
            return Err(FSEIndexError::EmptyNodeList);
        }

        if root >= nodes.len() {
            return Err(FSEIndexError::MissingRoot {
                root,
                node_count: nodes.len(),
            });
        }

        let dimensions = nodes[root].dimensions();
        for (node_id, node) in nodes.iter().enumerate() {
            if node.dimensions() != dimensions {
                return Err(FSEIndexError::DimensionMismatch {
                    node: node_id,
                    actual_dimensions: node.dimensions(),
                    expected_dimensions: dimensions,
                });
            }
        }

        let mut leaf_node_ids = Vec::new();
        let mut leaf_reconstruction_shapes = Vec::new();
        let mut leaf_reconstruction_shapes_by_node = vec![None; nodes.len()];

        for (node_id, node) in nodes.iter().enumerate() {
            if !node.is_leaf {
                continue;
            }

            let shape = LeafReconstructionShape::new(
                node_id,
                node.dimensions(),
                node.residuals.cardinality(),
            );

            leaf_node_ids.push(node_id);
            leaf_reconstruction_shapes.push(shape);
            leaf_reconstruction_shapes_by_node[node_id] = Some(shape);
        }

        let leaf_count = leaf_node_ids.len();

        Ok(Self {
            nodes,
            root,
            dimensions,
            leaf_count,
            leaf_node_ids,
            leaf_reconstruction_shapes,
            leaf_reconstruction_shapes_by_node,
        })
    }

    /// Creates a single-leaf index from a root partition.
    pub fn from_root(root: PartitionNode) -> Self {
        Self::new(vec![root], 0)
    }

    /// Creates a single-leaf index from a root partition.
    ///
    /// This is the checked counterpart to [`FSEIndex::from_root`].
    pub fn try_from_root(root: PartitionNode) -> Result<Self, FSEIndexError> {
        Self::try_new(vec![root], 0)
    }

    /// Returns the root partition.
    pub fn root_node(&self) -> &PartitionNode {
        &self.nodes[self.root]
    }

    /// Returns the number of nodes in the index.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of leaf partitions in the index.
    ///
    /// # Runtime Role
    ///
    /// This value is cached during construction so query traversal can report
    /// leaf pruning statistics without scanning all nodes first.
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Returns leaf node identifiers in node-list order.
    ///
    /// # Runtime Role
    ///
    /// This is the cached leaf iteration path for root-covered execution and
    /// other execution helpers that need every leaf.
    pub fn leaf_node_ids(&self) -> &[usize] {
        &self.leaf_node_ids
    }

    /// Returns cached reconstruction shapes for all leaves.
    ///
    /// # Runtime Role
    ///
    /// This supports full-root coverage paths that need to reconstruct every
    /// leaf and already know every leaf is covered.
    pub fn leaf_reconstruction_shapes(&self) -> &[LeafReconstructionShape] {
        &self.leaf_reconstruction_shapes
    }

    /// Returns cached reconstruction shape for a leaf node.
    ///
    /// # Runtime Role
    ///
    /// Retained-leaf execution receives leaf node ids from traversal. This method
    /// maps those ids directly to cached reconstruction metadata.
    ///
    /// # Panics
    ///
    /// Panics when `node_id` is outside the index or does not reference a leaf.
    pub fn leaf_reconstruction_shape(&self, node_id: usize) -> LeafReconstructionShape {
        self.leaf_reconstruction_shapes_by_node
            .get(node_id)
            .copied()
            .flatten()
            .unwrap_or_else(|| panic!("node id {node_id} does not reference a leaf partition"))
    }

    /// Returns the number of internal partitions in the index.
    pub fn internal_node_count(&self) -> usize {
        self.node_count() - self.leaf_count
    }

    /// Returns true when the index contains no hierarchy below the root.
    pub fn is_single_leaf(&self) -> bool {
        self.nodes.len() == 1 && self.root_node().is_leaf
    }
}
