//! Global FSE index hierarchy.

use crate::storage::PartitionNode;

/// In-memory representation of an FSE hierarchy.
///
/// # Runtime Role
///
/// `FSEIndex` owns all partition nodes and identifies the root of the hierarchy.
/// Query execution traverses this structure without mutating it.
///
/// The index also caches simple structural counts that are needed during query
/// reporting. These values are derived once at construction time so query
/// execution does not need to rescan the whole node list for every request.
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

        let leaf_count = nodes.iter().filter(|node| node.is_leaf).count();

        Self {
            nodes,
            root,
            dimensions,
            leaf_count,
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

    /// Returns the number of internal partitions in the index.
    pub fn internal_node_count(&self) -> usize {
        self.node_count() - self.leaf_count
    }

    /// Returns true when the index contains no hierarchy below the root.
    pub fn is_single_leaf(&self) -> bool {
        self.nodes.len() == 1 && self.root_node().is_leaf
    }
}
