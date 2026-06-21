//! Typed query execution through planning decisions.

use crate::data::RowId;

use super::append_delta::TypedAppendDeltaQueryView;
use super::execution::{
    IndexedTypedQueryError, count_typed_query_matches, evaluate_typed_query_plan,
    typed_query_has_match,
};
use super::index::TypedQueryIndex;
use super::plan::TypedQueryPlan;
use super::planning::{
    TypedQueryExecutionStrategy, TypedQueryOutputContract, TypedQueryPlanningDiagnostics,
    plan_typed_append_delta_query_execution, plan_typed_query_execution,
};

/// Row-id query result paired with planning diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedTypedQueryRowIdReport {
    /// Matching logical row identifiers.
    pub row_ids: Vec<RowId>,

    /// Planning diagnostics used to choose the execution path.
    pub diagnostics: TypedQueryPlanningDiagnostics,
}

/// Count query result paired with planning diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedTypedQueryCountReport {
    /// Number of matching records.
    pub matched_records: usize,

    /// Planning diagnostics used to choose the execution path.
    pub diagnostics: TypedQueryPlanningDiagnostics,
}

/// Existence query result paired with planning diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedTypedQueryExistenceReport {
    /// Whether at least one record matched the query plan.
    pub has_match: bool,

    /// Planning diagnostics used to choose the execution path.
    pub diagnostics: TypedQueryPlanningDiagnostics,
}

/// Evaluates row-id output using the typed query planner.
pub fn planned_typed_query_row_ids(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryRowIdReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::RowIds);
    let row_ids = execute_index_strategy(index, plan, diagnostics.strategy)?;

    Ok(PlannedTypedQueryRowIdReport {
        row_ids,
        diagnostics,
    })
}

/// Evaluates count output using the typed query planner.
pub fn planned_typed_query_count_matches(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryCountReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::Count);
    let matched_records = execute_index_count_strategy(index, plan, diagnostics.strategy)?;

    Ok(PlannedTypedQueryCountReport {
        matched_records,
        diagnostics,
    })
}

/// Evaluates existence output using the typed query planner.
pub fn planned_typed_query_has_match(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryExistenceReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::Existence);
    let has_match = execute_index_existence_strategy(index, plan, diagnostics.strategy)?;

    Ok(PlannedTypedQueryExistenceReport {
        has_match,
        diagnostics,
    })
}

/// Evaluates append-delta row-id output using the typed query planner.
pub fn planned_append_delta_query_row_ids(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryRowIdReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::RowIds);
    let row_ids = execute_append_delta_strategy(view, plan, diagnostics.strategy)?;

    Ok(PlannedTypedQueryRowIdReport {
        row_ids,
        diagnostics,
    })
}

/// Evaluates append-delta count output using the typed query planner.
pub fn planned_append_delta_query_count_matches(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryCountReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::Count);
    let matched_records = execute_append_delta_count_strategy(view, plan, diagnostics.strategy)?;

    Ok(PlannedTypedQueryCountReport {
        matched_records,
        diagnostics,
    })
}

/// Evaluates append-delta existence output using the typed query planner.
pub fn planned_append_delta_query_has_match(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryExistenceReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::Existence);
    let has_match = execute_append_delta_existence_strategy(view, plan, diagnostics.strategy)?;

    Ok(PlannedTypedQueryExistenceReport {
        has_match,
        diagnostics,
    })
}

fn execute_index_strategy(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
) -> Result<Vec<RowId>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => Ok(evaluate_typed_query_plan(index.batch(), plan)),
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.query_row_ids(plan)
        }
    }
}

fn execute_index_count_strategy(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
) -> Result<usize, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(0),
        TypedQueryExecutionStrategy::FlatScan => Ok(count_typed_query_matches(index.batch(), plan)),
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.count_matches(plan)
        }
    }
}

fn execute_index_existence_strategy(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
) -> Result<bool, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(false),
        TypedQueryExecutionStrategy::FlatScan => Ok(typed_query_has_match(index.batch(), plan)),
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.has_match(plan)
        }
    }
}

fn execute_append_delta_strategy(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
) -> Result<Vec<RowId>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => {
            let mut row_ids = evaluate_typed_query_plan(view.base().batch(), plan);
            row_ids.extend(evaluate_typed_query_plan(view.appended(), plan));
            Ok(row_ids)
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.query_row_ids(plan)
        }
    }
}

fn execute_append_delta_count_strategy(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
) -> Result<usize, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(0),
        TypedQueryExecutionStrategy::FlatScan => {
            Ok(count_typed_query_matches(view.base().batch(), plan)
                + count_typed_query_matches(view.appended(), plan))
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.count_matches(plan)
        }
    }
}

fn execute_append_delta_existence_strategy(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
) -> Result<bool, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(false),
        TypedQueryExecutionStrategy::FlatScan => {
            Ok(typed_query_has_match(view.base().batch(), plan)
                || typed_query_has_match(view.appended(), plan))
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.has_match(plan)
        }
    }
}
