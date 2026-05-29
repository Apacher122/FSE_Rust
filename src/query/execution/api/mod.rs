//! Public query execution API.
//!
//! This module owns the public owned-result query APIs and the internal
//! diagnostic retained-leaf execution seam. Fresh owned-result execution,
//! reusable-buffer execution, and diagnostic execution are split by responsibility.

mod diagnostics;
mod owned;
mod reusable;

pub use owned::{
    execute_query, execute_query_with_options, execute_query_with_stats,
    execute_query_with_stats_and_options,
};

pub use reusable::{execute_query_into, execute_query_into_with_options};

pub(crate) use diagnostics::execute_retained_leaf_batch_for_diagnostics;
