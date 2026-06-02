//! Recursive FSE index builder.
//!
//! This folder module keeps the public builder API stable while separating
//! configuration, construction, split acceptance, and builder result types.

mod acceptance;
mod config;
mod construction;
mod types;

pub use acceptance::accepts_split_quality;
pub use config::BuildConfig;
pub use construction::FSEBuilder;
pub use types::{BuildValidationError, ValidatedFSEIndex};
