//! Reference-result reconstruction.
//!
//! This module separates the public deferred-reconstruction API from reference
//! validation helpers while preserving the existing reference-result API.

mod api;
mod validation;

pub use api::{
    reconstruct_query_result_reference, reconstruct_query_result_reference_into,
    reconstruct_query_result_references, reconstruct_query_result_references_into,
};
