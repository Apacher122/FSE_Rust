//! Global FSE index hierarchy.

use crate::storage::PartitionNode;

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
        assert!(!nodes.is_empty(), "index must contain at least one node");
        assert!(
            root < nodes.len(),
            "root node id must exist in the node list"
        );

        let dimensions = nodes[root].dimensions();
        for node in &nodes {
            assert_eq!(
                node.dimensions(),
                dimensions,
                "all nodes in an index must have the same dimensionality"
            );
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

        Self {
            nodes,
            root,
            dimensions,
            leaf_count,
            leaf_node_ids,
            leaf_reconstruction_shapes,
            leaf_reconstruction_shapes_by_node,
        }
    }

    /// Creates a single-leaf index from a root partition.
    pub fn from_root(root: PartitionNode) -> Self {
        Self::new(vec![root], 0)
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
