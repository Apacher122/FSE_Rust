//! Query execution components.
//!
//! This module contains query region definitions and the staged execution logic
//! used by the FSE runtime.

pub mod region;
pub mod traversal;

pub use region::QueryRegion;
pub use traversal::traverse;
