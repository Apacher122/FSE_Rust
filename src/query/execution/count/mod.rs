//! Count-only query execution.
//!
//! Count-only execution preserves the same geometric pruning and exact
//! predicate semantics as owned-result execution, but it does not allocate
//! returned `Vector` values for matching rows.

mod api;
mod execution;
mod stats;

pub use api::{count_query_matches, count_query_matches_with_stats};

pub(crate) use execution::count_retained_matches_without_results;
