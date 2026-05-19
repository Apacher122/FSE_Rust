//! Retained-leaf execution.
//!
//! This module contains the Stage II and Stage III retained-leaf execution
//! helpers used after geometric traversal has selected candidate leaves.

mod dispatch;
mod leaf;
mod merge;

#[cfg(any(test, debug_assertions))]
mod validation;

#[cfg(test)]
mod test_support;

pub(crate) use dispatch::execute_classified_retained_leaves_with_candidate_count;

pub(crate) use leaf::{
    append_covered_retained_leaf_results, execute_retained_leaf_into_batch_report,
    execute_retained_leaf_with_cached_shape,
};

pub(crate) use merge::merge_retained_leaf_reports_in_order;

#[cfg(test)]
pub(crate) use merge::{merge_retained_leaf_report, reserve_additional_results};

#[cfg(any(test, debug_assertions))]
pub(crate) use validation::validate_retained_leaves;

#[cfg(test)]
pub(crate) use test_support::{
    classified_retained_candidate_count, execute_classified_retained_leaves_with_options,
    execute_covered_retained_leaf, execute_partially_covered_retained_leaf, execute_retained_leaf,
    execute_retained_leaves, execute_retained_leaves_parallel, execute_retained_leaves_serial,
    execute_retained_leaves_with_options, query_contains_bounds, retained_candidate_count,
    validate_retained_leaf_ids,
};
