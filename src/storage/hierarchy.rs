//! Global FSE index hierarchy.

use crate::storage::PartitionNode;

/// An in-memory representation of a Fractal Semantic Encoding hierarchy.
///
/// `FSEIndex` serves as the primary owner of all partition nodes and identifies the
/// entry point (root) of the searchable space. During query execution, this structure
/// is traversed as a read-only topology to identify relevant data regions. In the
/// formal FSE specification, this structure corresponds to the global representation
/// $\mathcal{F} = \{P_1, P_2, \dots, P_K\}$.
#[derive(Clone, Debug, PartialEq)]
pub struct FSEIndex {
    /// The collection of all partition nodes comprising the hierarchy.
    pub nodes: Vec<PartitionNode>,
    /// The unique identifier of the root partition.
    pub root: usize,
    /// The dimensionality of the coordinate space represented by this index.
    pub dimensions: usize,
}

impl FSEIndex {
    /// Creates a new hierarchy index from a collection of partition nodes and a root identifier.
    ///
    /// # Panics
    ///
    /// Panics if the node collection is empty, the provided root identifier does not
    /// exist within the collection, or if any node possesses a dimensionality that
    /// differs from the root partition.
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

        Self {
            nodes,
            root,
            dimensions,
        }
    }

    /// Constructs a simplified, single-leaf index from a single root partition.
    pub fn from_root(root: PartitionNode) -> Self {
        Self::new(vec![root], 0)
    }

    /// Returns an immutable reference to the root partition node.
    pub fn root_node(&self) -> &PartitionNode {
        &self.nodes[self.root]
    }

    /// Returns the total number of nodes currently managed by the index.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Checks if the index consists solely of a single leaf node with no descendants.
    pub fn is_single_leaf(&self) -> bool {
        self.nodes.len() == 1 && self.root_node().is_leaf
    }
}
