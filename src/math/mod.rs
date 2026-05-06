//! Mathematical primitives used by the FSE runtime.
//!
//! This module contains the coordinate, bounding, centroid, and residual
//! structures that correspond to the formal model of FSE.
pub mod bounds;
pub mod centroid;
pub mod residuals;
pub mod vector;

pub use bounds::BoundingBox;
pub use centroid::compute_centroid;
pub use residuals::ResidualBlock;
pub use vector::{Scalar, Vector};
