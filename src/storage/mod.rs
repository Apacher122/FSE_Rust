//! Storage structures for the FSE runtime.
//!
//! This module contains the partition and hierarchy types used to represent
//! the FSE index in memory.

pub mod hierarchy;
pub mod partition;

pub use hierarchy::{FSEIndex, FSEIndexError, LeafReconstructionShape};
pub use partition::{PartitionFromPointsError, PartitionNode};
