//! Axis-aligned bounding regions.
//!
//! This module keeps bounding box storage, construction, spatial predicates,
//! and metrics separated while preserving the public `BoundingBox` API.

mod box_shape;
mod construction;
mod metrics;
mod spatial;

pub use self::box_shape::BoundingBox;
pub use self::construction::BoundingBoxError;
