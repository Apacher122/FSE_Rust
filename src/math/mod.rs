//! Mathematical primitives used by the FSE runtime.
//!
//! This module contains the coordinate, bounding, centroid, and residual
//! structures that correspond to the formal model of FSE.
pub mod bounds;
pub mod centroid;
pub mod residuals;
pub mod vector;

pub use bounds::{BoundingBox, BoundingBoxError};
pub use centroid::{CentroidError, compute_centroid, try_compute_centroid};
pub use residuals::{ResidualBlock, ResidualBlockError};
pub use vector::{Scalar, Vector, VectorError};
