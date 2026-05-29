//! Residual reconstruction.
//!
//! This module separates reconstruction shape validation from row, point, and
//! partition materialization while preserving the public reconstruction API.

mod partition;
mod point;
mod row;
mod shape;

pub use self::partition::reconstruct_partition;
pub use self::point::reconstruct_point;
pub use self::row::reconstruct_row_into;

pub(crate) use self::row::reconstruct_row_into_prevalidated;

#[cfg(test)]
pub(crate) use self::shape::validate_partition_reconstruction_shape;
