//! Query execution components.
//!
//! This module contains query region definitions and the staged execution logic
//! used by the FSE runtime.
//!
//! The public query APIs expose several output contracts over the same exact
//! result set. Owned-result, reusable owned-result, reference-result,
//! reference-visitor, row-view visitor, count-only, and existence execution all
//! use the same geometric traversal and exact predicate semantics. They differ
//! only in how accepted rows are delivered to the caller.
//!
//! Use owned-result execution when callers need materialized
//! [`crate::math::Vector`] values. Use reference or visitor execution when
//! callers can defer row materialization. Use count-only or existence execution
//! when callers need cardinality or non-empty-result information instead of
//! rows.

pub mod evaluator;
pub mod execution;
pub mod predicate;
pub mod reconstruction;
pub mod region;
pub mod traversal;

pub use evaluator::evaluate_query;
pub use execution::{
    QueryCountReport, QueryExecutionMode, QueryExecutionOptions, QueryExecutionReport,
    QueryExecutionStats, QueryExistenceReport, QueryReferenceReport, QueryResultReference,
    QueryResultRowView, count_query_matches, count_query_matches_with_stats, execute_query,
    execute_query_into, execute_query_into_with_options, execute_query_references,
    execute_query_references_with_stats, execute_query_with_options, execute_query_with_stats,
    execute_query_with_stats_and_options, query_has_match, query_has_match_with_stats,
    query_result_row_view, reconstruct_query_result_reference,
    reconstruct_query_result_reference_into, reconstruct_query_result_references,
    reconstruct_query_result_references_into, visit_query_references, visit_query_row_views,
};

pub(crate) use execution::{
    count_retained_matches_without_results, execute_retained_leaf_batch_for_diagnostics,
};
pub use predicate::{
    FSEPredicate, FSEPredicateError, FSEPredicateField, FSEPredicateOperator,
    ValidatedFSEPredicate, ValidatedFSEPredicateOperator,
};
pub use reconstruction::{reconstruct_partition, reconstruct_point, reconstruct_row_into};
pub use region::{QueryRegion, QueryRegionError};
pub use traversal::{
    QueryTraversalReport, QueryTraversalStats, RetainedLeaf, RetainedLeafCoverage, traverse,
    traverse_with_stats,
};
