//! Query execution runtime.
//!
//! This module contains the staged FSE query runtime. The public API lives in
//! `api`, execution configuration lives in `options`, execution report types
//! live in `reports`, and the serial, parallel, retained-leaf, and root-covered
//! execution paths are split by responsibility.

mod api;
mod count;
mod options;
mod parallel;
mod references;
mod reports;
mod retained;
mod root_coverage;
mod serial;

pub use api::{
    execute_query, execute_query_into, execute_query_into_with_options, execute_query_with_options,
    execute_query_with_stats, execute_query_with_stats_and_options,
};

pub use count::{count_query_matches, count_query_matches_with_stats};

pub use references::{
    execute_query_references, execute_query_references_with_stats,
    reconstruct_query_result_reference, reconstruct_query_result_reference_into,
    reconstruct_query_result_references, reconstruct_query_result_references_into,
};

pub(crate) use api::execute_retained_leaf_batch_for_diagnostics;

pub use options::{
    DEFAULT_PARALLEL_MIN_RETAINED_LEAVES, QueryExecutionMode, QueryExecutionOptions,
};

pub use reports::{
    QueryCountReport, QueryExecutionReport, QueryExecutionStats, QueryReferenceReport,
    QueryResultReference,
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
