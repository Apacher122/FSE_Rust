//! Global FSE index hierarchy.

use crate::storage::PartitionNode;

/// In-memory representation of an FSE hierarchy.
///
/// # Runtime Role
///
/// `FSEIndex` owns all partition nodes and identifies the root of the hierarchy.
/// Query execution traverses this structure without mutating it.
///
/// The index also caches simple structural counts and leaf identifiers that are
/// needed during query execution. These values are derived once at construction
/// time so hot query paths do not need to rescan the whole node list for every
/// request.
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

        let leaf_node_ids = nodes
            .iter()
            .enumerate()
            .filter_map(
                |(node_id, node)| {
                    if node.is_leaf { Some(node_id) } else { None }
                },
            )
            .collect::<Vec<_>>();

        let leaf_count = leaf_node_ids.len();

        Self {
            nodes,
            root,
            dimensions,
            leaf_count,
            leaf_node_ids,
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

    /// Returns the number of internal partitions in the index.
    pub fn internal_node_count(&self) -> usize {
        self.node_count() - self.leaf_count
    }

    /// Returns true when the index contains no hierarchy below the root.
    pub fn is_single_leaf(&self) -> bool {
        self.nodes.len() == 1 && self.root_node().is_leaf
    }
}
