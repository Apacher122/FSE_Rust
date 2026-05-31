//! Reference-result reconstruction.
//!
//! This module separates the public deferred-reconstruction API from reference
//! validation helpers while preserving the existing reference-result API.

mod api;
mod validation;

pub use api::{
    QueryResultRowView, query_result_row_view, reconstruct_query_result_reference,
    reconstruct_query_result_reference_into, reconstruct_query_result_references,
    reconstruct_query_result_references_into,
};
