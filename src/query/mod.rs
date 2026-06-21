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

pub mod execution;
pub mod reconstruction;
pub mod region;
pub mod traversal;
pub mod typed;

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
pub use reconstruction::{reconstruct_partition, reconstruct_point, reconstruct_row_into};
pub use region::{QueryRegion, QueryRegionError, evaluate_query};
pub use traversal::{
    QueryTraversalReport, QueryTraversalStats, RetainedLeaf, RetainedLeafCoverage, traverse,
    traverse_with_stats,
};
pub use typed::{
    FSEPredicate, FSEPredicateCompileError, FSEPredicateError, FSEPredicateField,
    FSEPredicateOperator, IndexedTypedQueryError, IndexedTypedQueryReport,
    IndexedTypedQueryRowReport, PlannedTypedQueryRowIdReport, TypedAppendDeltaQueryView,
    TypedQueryExecutionStrategy, TypedQueryIndex, TypedQueryIndexAppendError,
    TypedQueryIndexBuildError, TypedQueryOutputContract, TypedQueryPlan, TypedQueryPlanBuilder,
    TypedQueryPlanError, TypedQueryPlanningDiagnostics, TypedQueryPlanningReason,
    TypedQueryResultRow, TypedRowTombstoneSet, ValidatedFSEPredicate,
    ValidatedFSEPredicateOperator, compile_categorical_equality_predicate_to_query_region,
    compile_numeric_predicate_to_query_region, count_indexed_typed_query_matches,
    count_indexed_typed_query_matches_excluding_tombstones,
    count_indexed_typed_query_matches_with_stats,
    count_indexed_typed_query_matches_with_stats_excluding_tombstones, count_typed_query_matches,
    evaluate_indexed_typed_query_plan, evaluate_indexed_typed_query_plan_excluding_tombstones,
    evaluate_indexed_typed_query_plan_rows,
    evaluate_indexed_typed_query_plan_rows_excluding_tombstones,
    evaluate_indexed_typed_query_plan_rows_with_stats,
    evaluate_indexed_typed_query_plan_rows_with_stats_excluding_tombstones,
    evaluate_indexed_typed_query_plan_with_stats,
    evaluate_indexed_typed_query_plan_with_stats_excluding_tombstones, evaluate_typed_predicate,
    evaluate_typed_query_plan, evaluate_typed_query_plan_rows, indexed_typed_query_has_match,
    indexed_typed_query_has_match_excluding_tombstones, indexed_typed_query_has_match_with_stats,
    indexed_typed_query_has_match_with_stats_excluding_tombstones,
    plan_typed_append_delta_query_execution, plan_typed_query_execution,
    planned_append_delta_query_row_ids, planned_typed_query_row_ids, typed_query_has_match,
    visit_indexed_typed_query_row_ids, visit_indexed_typed_query_row_ids_excluding_tombstones,
    visit_indexed_typed_query_rows, visit_indexed_typed_query_rows_excluding_tombstones,
};
