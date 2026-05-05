//! Global FSE index hierarchy.

use crate::storage::PartitionNode;

#[derive(Clone, Debug, PartialEq)]
pub struct FSEIndex {
    pub nodes: Vec<PartitionNode>,
    pub root: usize,
    pub dimensions: usize,
}

impl FSEIndex {
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

    pub fn from_root(root: PartitionNode) -> Self {
        Self::new(vec![root], 0)
    }

    pub fn root_node(&self) -> &PartitionNode {
        &self.nodes[self.root]
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_single_leaf(&self) -> bool {
        self.nodes.len() == 1 && self.root_node().is_leaf
    }
}
