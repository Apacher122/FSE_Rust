//! Reference-result query execution.
//!
//! This module provides an exact query output contract that returns references
//! to matching residual rows instead of materializing owned `Vector` values.
//! Query execution, retained-leaf matching, and deferred reference reconstruction
//! are split by responsibility.

mod execution;
pub(in crate::query::execution) mod matching;
mod reconstruction;

pub use execution::{execute_query_references, execute_query_references_with_stats};

pub use reconstruction::{
    QueryResultRowView, query_result_row_view, reconstruct_query_result_reference,
    reconstruct_query_result_reference_into, reconstruct_query_result_references,
    reconstruct_query_result_references_into,
};
