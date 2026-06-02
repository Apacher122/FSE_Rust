//! Centroid-relative residual storage.
//!
//! This module keeps residual block storage, construction, and shape helpers
//! separated while preserving the public `ResidualBlock` API.

mod block;
mod construction;
mod shape;

pub use self::block::ResidualBlock;
pub use self::construction::ResidualBlockError;
