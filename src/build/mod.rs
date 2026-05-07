//! Index construction components.
//!
//! This module contains the initial builder pipeline for constructing an FSE
//! hierarchy from embedded coordinate vectors.

pub mod builder;
pub mod metrics;
pub mod splitter;
pub mod validation;
pub mod variance;

pub use builder::{BuildConfig, FSEBuilder};
pub use metrics::{index_density, partition_density};
pub use validation::{IndexValidationReport, validate_index};
