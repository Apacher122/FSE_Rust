//! Partition node storage.
//!
//! This module contains the local partition representation used by the FSE
//! hierarchy. The public API remains `PartitionNode`; the construction logic is
//! split away from the raw node shape so storage responsibilities stay easier to
//! audit as the runtime grows.

mod construction;
mod node;

pub use node::PartitionNode;
