//! Metadata traversal for geometric pruning.
//!
//! This module implements Stage I of FSE query execution. Traversal decides
//! which leaf partitions are geometrically admissible before reconstruction and
//! exact predicate evaluation run.

mod covered;
mod execution;
mod report;
mod retained_leaf;
mod retention;
mod stack;

pub(crate) use execution::traverse_with_known_root_classification;
pub use execution::{traverse, traverse_with_stats};

pub use report::{QueryTraversalReport, QueryTraversalStats};

pub use retained_leaf::{RetainedLeaf, RetainedLeafCoverage};
