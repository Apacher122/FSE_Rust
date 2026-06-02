//! Query execution runtime.
//!
//! This module contains the staged FSE query runtime. The public API lives in
//! `api`, execution configuration lives in `options`, execution report types
//! live in `reports`, and the serial, parallel, retained-leaf, and root-covered
//! execution paths are split by responsibility.
//!
//! All public execution functions preserve the same exact query semantics:
//!
//! ```text
//! Geometry -> Reconstruction -> Logic
//! ```
//!
//! The output contract determines what is returned after exact predicate
//! evaluation:
//!
//! - owned-result APIs return materialized [`crate::math::Vector`] values;
//! - reusable owned-result APIs write materialized rows into a caller buffer;
//! - reference-result APIs return [`QueryResultReference`] values;
//! - visitor APIs stream references or borrowed row views;
//! - count-only APIs return exact cardinality;
//! - existence APIs return whether the exact result set is non-empty.

mod api;
mod count;
mod exists;
#[cfg(any(test, debug_assertions))]
mod leaf_shape_debug;
mod options;
mod parallel;
mod ratio;
mod references;
mod reports;
mod retained;
mod root;
mod root_coverage;
mod serial;
mod stats;
mod visit;

pub use api::{
    execute_query, execute_query_into, execute_query_into_with_options, execute_query_with_options,
    execute_query_with_stats, execute_query_with_stats_and_options,
};

pub use count::{count_query_matches, count_query_matches_with_stats};

pub use exists::{query_has_match, query_has_match_with_stats};

pub use visit::{visit_query_references, visit_query_row_views};

pub use references::{
    QueryResultRowView, execute_query_references, execute_query_references_with_stats,
    query_result_row_view, reconstruct_query_result_reference,
    reconstruct_query_result_reference_into, reconstruct_query_result_references,
    reconstruct_query_result_references_into,
};

pub(crate) use api::execute_retained_leaf_batch_for_diagnostics;
pub(crate) use count::count_retained_matches_without_results;

pub use options::{
    DEFAULT_PARALLEL_MIN_RETAINED_LEAVES, QueryExecutionMode, QueryExecutionOptions,
};

pub use reports::{
    QueryCountReport, QueryExecutionReport, QueryExecutionStats, QueryExistenceReport,
    QueryReferenceReport, QueryResultReference,
};

#[cfg(test)]
pub(crate) use parallel::should_execute_retained_leaves_in_parallel;

#[cfg(test)]
pub(crate) use reports::{
    MAX_RESULT_PREALLOCATION, RetainedLeafExecutionReport, result_capacity_hint,
};

#[cfg(test)]
pub(crate) use retained::{
    execute_classified_retained_leaves_with_options, execute_covered_retained_leaf,
    execute_partially_covered_retained_leaf, execute_retained_leaf, execute_retained_leaves,
    execute_retained_leaves_parallel, execute_retained_leaves_serial,
    execute_retained_leaves_with_options, merge_retained_leaf_report,
    merge_retained_leaf_reports_in_order, query_contains_bounds, reserve_additional_results,
    retained_candidate_count, validate_retained_leaf_ids,
};

#[cfg(test)]
pub(crate) use root_coverage::{
    execute_fully_covered_index_serial, execute_fully_covered_index_with_options,
    fully_covered_retained_leaves, leaf_count,
};

#[cfg(test)]
pub(crate) use serial::execute_classified_retained_leaves_serial;
