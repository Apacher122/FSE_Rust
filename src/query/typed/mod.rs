//! Typed query planning and execution.

pub mod append_delta;
pub mod builder;
pub mod compiler;
pub mod evaluator;
pub mod execution;
pub mod index;
pub mod plan;
pub mod planned_execution;
pub mod planning;
pub mod predicate;
pub mod tombstone;

pub use append_delta::{TypedAppendDeltaQueryView, TypedTombstonedAppendDeltaQueryView};
pub use builder::TypedQueryPlanBuilder;
pub use compiler::{
    FSEPredicateCompileError, compile_categorical_equality_predicate_to_query_region,
    compile_numeric_predicate_to_query_region,
};
pub use evaluator::evaluate_typed_predicate;
pub use execution::{
    IndexedTypedQueryError, IndexedTypedQueryReport, IndexedTypedQueryRowReport,
    TypedQueryResultRow, count_indexed_typed_query_matches,
    count_indexed_typed_query_matches_excluding_tombstones,
    count_indexed_typed_query_matches_with_stats,
    count_indexed_typed_query_matches_with_stats_excluding_tombstones, count_typed_query_matches,
    evaluate_indexed_typed_query_plan, evaluate_indexed_typed_query_plan_excluding_tombstones,
    evaluate_indexed_typed_query_plan_rows,
    evaluate_indexed_typed_query_plan_rows_excluding_tombstones,
    evaluate_indexed_typed_query_plan_rows_with_stats,
    evaluate_indexed_typed_query_plan_rows_with_stats_excluding_tombstones,
    evaluate_indexed_typed_query_plan_with_stats,
    evaluate_indexed_typed_query_plan_with_stats_excluding_tombstones, evaluate_typed_query_plan,
    evaluate_typed_query_plan_rows, indexed_typed_query_has_match,
    indexed_typed_query_has_match_excluding_tombstones, indexed_typed_query_has_match_with_stats,
    indexed_typed_query_has_match_with_stats_excluding_tombstones, typed_query_has_match,
    visit_indexed_typed_query_row_ids, visit_indexed_typed_query_row_ids_excluding_tombstones,
    visit_indexed_typed_query_rows, visit_indexed_typed_query_rows_excluding_tombstones,
};
pub use index::{TypedQueryIndex, TypedQueryIndexAppendError, TypedQueryIndexBuildError};
pub use plan::{TypedQueryPlan, TypedQueryPlanError};
pub use planned_execution::{
    PlannedTypedQueryCountReport, PlannedTypedQueryExistenceReport, PlannedTypedQueryRowIdReport,
    PlannedTypedQueryRowReport, PlannedTypedQueryVisitReport,
    planned_append_delta_query_count_matches,
    planned_append_delta_query_count_matches_excluding_tombstones,
    planned_append_delta_query_has_match,
    planned_append_delta_query_has_match_excluding_tombstones, planned_append_delta_query_row_ids,
    planned_append_delta_query_row_ids_excluding_tombstones, planned_append_delta_query_rows,
    planned_append_delta_query_rows_excluding_tombstones, planned_append_delta_query_visit_row_ids,
    planned_append_delta_query_visit_row_ids_excluding_tombstones,
    planned_append_delta_query_visit_rows,
    planned_append_delta_query_visit_rows_excluding_tombstones, planned_typed_query_count_matches,
    planned_typed_query_count_matches_excluding_tombstones, planned_typed_query_has_match,
    planned_typed_query_has_match_excluding_tombstones, planned_typed_query_row_ids,
    planned_typed_query_row_ids_excluding_tombstones, planned_typed_query_rows,
    planned_typed_query_rows_excluding_tombstones, planned_typed_query_visit_row_ids,
    planned_typed_query_visit_row_ids_excluding_tombstones, planned_typed_query_visit_rows,
    planned_typed_query_visit_rows_excluding_tombstones,
};
pub use planning::{
    TypedQueryExecutionStrategy, TypedQueryOutputContract, TypedQueryPlanningDiagnostics,
    TypedQueryPlanningReason, plan_typed_append_delta_query_execution, plan_typed_query_execution,
};
pub use predicate::{
    FSEPredicate, FSEPredicateError, FSEPredicateField, FSEPredicateOperator,
    ValidatedFSEPredicate, ValidatedFSEPredicateOperator,
};
pub use tombstone::{TypedRowTombstoneSet, TypedTombstonedQueryIndexView};
