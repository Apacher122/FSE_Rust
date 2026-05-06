//! Index construction components.
//!
//! This module contains the initial builder pipeline for constructing an FSE
//! hierarchy from embedded coordinate vectors.

pub mod builder;
pub mod splitter;
pub mod variance;

pub use builder::{BuildConfig, FSEBuilder};
