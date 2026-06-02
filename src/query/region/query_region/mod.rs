//! Axis-aligned query region behavior.
//!
//! This module separates query region storage, construction, bounds
//! classification, and exact point predicate helpers while preserving the public
//! `QueryRegion` API.

mod bounds_classification;
mod construction;
mod point_predicate;
mod shape;

pub use self::construction::QueryRegionError;
pub use self::shape::QueryRegion;
