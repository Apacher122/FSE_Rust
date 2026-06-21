//! Typed query execution through planning decisions.

use crate::data::RowId;

use super::append_delta::TypedAppendDeltaQueryView;
use super::execution::{IndexedTypedQueryError, evaluate_typed_query_plan};
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
