//! Typed query execution through planning decisions.

use crate::data::{FSERecord, FSERecordBatch, RowId};

use super::append_delta::TypedAppendDeltaQueryView;
use super::execution::{
    IndexedTypedQueryError, TypedQueryResultRow, count_typed_query_matches,
    evaluate_typed_query_plan, evaluate_typed_query_plan_rows,
    evaluate_typed_query_plan_rows_with_capacity, evaluate_typed_query_plan_with_capacity,
    record_matches_plan, typed_query_has_match,
};
use super::index::TypedQueryIndex;
use super::plan::TypedQueryPlan;
use super::planning::{
    TypedQueryExecutionStrategy, TypedQueryOutputContract, TypedQueryPlanningDiagnostics,
    plan_typed_append_delta_query_execution, plan_typed_query_execution,
};
use super::tombstone::TypedRowTombstoneSet;

use crate::query::execution::QueryExecutionStats;

/// Row-id query result paired with planning diagnostics and execution statistics.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedTypedQueryRowIdReport {
    /// Matching logical row identifiers.
    pub row_ids: Vec<RowId>,

    /// Planning diagnostics used to choose the execution path.
    pub diagnostics: TypedQueryPlanningDiagnostics,

    /// Runtime statistics collected during query execution.
    pub execution_stats: QueryExecutionStats,
}

/// Typed row query result paired with planning diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedTypedQueryRowReport {
    /// Matching typed rows.
    pub rows: Vec<TypedQueryResultRow>,

    /// Planning diagnostics used to choose the execution path.
    pub diagnostics: TypedQueryPlanningDiagnostics,
}

/// Visitor query result paired with planning diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedTypedQueryVisitReport {
    /// Number of matches delivered to the visitor.
    pub visited_records: usize,

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
    let strategy = diagnostics.strategy;

    let (row_ids, execution_stats) = match strategy {
        TypedQueryExecutionStrategy::NoOp => (Vec::new(), QueryExecutionStats::default()),
        TypedQueryExecutionStrategy::FlatScan => {
            let ids = evaluate_typed_query_plan_with_capacity(
                index.batch(),
                plan,
                diagnostics.estimated_candidate_records,
            );
            let matched = ids.len();
            let total = index.batch().len();
            let stats = QueryExecutionStats {
                total_records: total,
                reconstructed_records: total,
                matched_records: matched,
                candidate_ratio: 1.0,
                ..QueryExecutionStats::default()
            };
            (ids, stats)
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            let report = index.query_row_ids_with_stats(plan)?;
            (report.row_ids, report.stats)
        }
    };

    Ok(PlannedTypedQueryRowIdReport {
        row_ids,
        diagnostics,
        execution_stats,
    })
}

/// Evaluates row-id output with tombstone filtering using the typed query planner.
pub fn planned_typed_query_row_ids_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryRowIdReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::RowIds);
    let row_ids =
        execute_index_strategy_excluding_tombstones(index, plan, tombstones, diagnostics.strategy)?;

    Ok(PlannedTypedQueryRowIdReport {
        row_ids,
        diagnostics,
        execution_stats: QueryExecutionStats::default(),
    })
}

/// Evaluates typed row output using the typed query planner.
pub fn planned_typed_query_rows(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryRowReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::Rows);
    let rows = execute_index_row_strategy(
        index,
        plan,
        diagnostics.strategy,
        diagnostics.estimated_candidate_records,
    )?;

    Ok(PlannedTypedQueryRowReport { rows, diagnostics })
}

/// Evaluates typed row output with tombstone filtering using the typed query planner.
pub fn planned_typed_query_rows_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryRowReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::Rows);
    let rows = execute_index_row_strategy_excluding_tombstones(
        index,
        plan,
        tombstones,
        diagnostics.strategy,
    )?;

    Ok(PlannedTypedQueryRowReport { rows, diagnostics })
}

/// Visits row identifiers using the typed query planner.
pub fn planned_typed_query_visit_row_ids<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let diagnostics =
        plan_typed_query_execution(index, plan, TypedQueryOutputContract::RowIdVisitor);
    let visited_records =
        execute_index_row_id_visitor_strategy(index, plan, diagnostics.strategy, visitor)?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
        diagnostics,
    })
}

/// Visits row identifiers with tombstone filtering using the typed query planner.
pub fn planned_typed_query_visit_row_ids_excluding_tombstones<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let diagnostics =
        plan_typed_query_execution(index, plan, TypedQueryOutputContract::RowIdVisitor);
    let visited_records = execute_index_row_id_visitor_strategy_excluding_tombstones(
        index,
        plan,
        tombstones,
        diagnostics.strategy,
        visitor,
    )?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
        diagnostics,
    })
}

/// Visits typed rows using the typed query planner.
pub fn planned_typed_query_visit_rows<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::RowVisitor);
    let visited_records =
        execute_index_row_visitor_strategy(index, plan, diagnostics.strategy, visitor)?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
        diagnostics,
    })
}

/// Visits typed rows with tombstone filtering using the typed query planner.
pub fn planned_typed_query_visit_rows_excluding_tombstones<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::RowVisitor);
    let visited_records = execute_index_row_visitor_strategy_excluding_tombstones(
        index,
        plan,
        tombstones,
        diagnostics.strategy,
        visitor,
    )?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
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

/// Evaluates count output with tombstone filtering using the typed query planner.
pub fn planned_typed_query_count_matches_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryCountReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::Count);
    let matched_records = execute_index_count_strategy_excluding_tombstones(
        index,
        plan,
        tombstones,
        diagnostics.strategy,
    )?;

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

/// Evaluates existence output with tombstone filtering using the typed query planner.
pub fn planned_typed_query_has_match_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryExistenceReport, IndexedTypedQueryError> {
    let diagnostics = plan_typed_query_execution(index, plan, TypedQueryOutputContract::Existence);
    let has_match = execute_index_existence_strategy_excluding_tombstones(
        index,
        plan,
        tombstones,
        diagnostics.strategy,
    )?;

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
        execution_stats: QueryExecutionStats::default(),
    })
}

/// Evaluates append-delta row-id output with tombstone filtering using the planner.
pub fn planned_append_delta_query_row_ids_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryRowIdReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::RowIds);
    let row_ids = execute_append_delta_strategy_excluding_tombstones(
        view,
        plan,
        tombstones,
        diagnostics.strategy,
    )?;

    Ok(PlannedTypedQueryRowIdReport {
        row_ids,
        diagnostics,
        execution_stats: QueryExecutionStats::default(),
    })
}

/// Evaluates append-delta typed row output using the typed query planner.
pub fn planned_append_delta_query_rows(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
) -> Result<PlannedTypedQueryRowReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::Rows);
    let rows = execute_append_delta_row_strategy(view, plan, diagnostics.strategy)?;

    Ok(PlannedTypedQueryRowReport { rows, diagnostics })
}

/// Evaluates append-delta typed row output with tombstone filtering using the planner.
pub fn planned_append_delta_query_rows_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryRowReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::Rows);
    let rows = execute_append_delta_row_strategy_excluding_tombstones(
        view,
        plan,
        tombstones,
        diagnostics.strategy,
    )?;

    Ok(PlannedTypedQueryRowReport { rows, diagnostics })
}

/// Visits append-delta row identifiers using the typed query planner.
pub fn planned_append_delta_query_visit_row_ids<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::RowIdVisitor);
    let visited_records =
        execute_append_delta_row_id_visitor_strategy(view, plan, diagnostics.strategy, visitor)?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
        diagnostics,
    })
}

/// Visits append-delta row identifiers with tombstone filtering using the planner.
pub fn planned_append_delta_query_visit_row_ids_excluding_tombstones<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::RowIdVisitor);
    let visited_records = execute_append_delta_row_id_visitor_strategy_excluding_tombstones(
        view,
        plan,
        tombstones,
        diagnostics.strategy,
        visitor,
    )?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
        diagnostics,
    })
}

/// Visits append-delta typed rows using the typed query planner.
pub fn planned_append_delta_query_visit_rows<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::RowVisitor);
    let visited_records =
        execute_append_delta_row_visitor_strategy(view, plan, diagnostics.strategy, visitor)?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
        diagnostics,
    })
}

/// Visits append-delta typed rows with tombstone filtering using the planner.
pub fn planned_append_delta_query_visit_rows_excluding_tombstones<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    visitor: F,
) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::RowVisitor);
    let visited_records = execute_append_delta_row_visitor_strategy_excluding_tombstones(
        view,
        plan,
        tombstones,
        diagnostics.strategy,
        visitor,
    )?;

    Ok(PlannedTypedQueryVisitReport {
        visited_records,
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

/// Evaluates append-delta count output with tombstone filtering using the planner.
pub fn planned_append_delta_query_count_matches_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryCountReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::Count);
    let matched_records = execute_append_delta_count_strategy_excluding_tombstones(
        view,
        plan,
        tombstones,
        diagnostics.strategy,
    )?;

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

/// Evaluates append-delta existence output with tombstone filtering using the planner.
pub fn planned_append_delta_query_has_match_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<PlannedTypedQueryExistenceReport, IndexedTypedQueryError> {
    let diagnostics =
        plan_typed_append_delta_query_execution(view, plan, TypedQueryOutputContract::Existence);
    let has_match = execute_append_delta_existence_strategy_excluding_tombstones(
        view,
        plan,
        tombstones,
        diagnostics.strategy,
    )?;

    Ok(PlannedTypedQueryExistenceReport {
        has_match,
        diagnostics,
    })
}

fn execute_index_strategy_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<Vec<RowId>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => {
            Ok(visible_row_ids_from_batch(index.batch(), plan, tombstones))
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.query_row_ids_excluding_tombstones(plan, tombstones)
        }
    }
}

fn execute_index_row_id_visitor_strategy<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            for row_id in evaluate_typed_query_plan(index.batch(), plan) {
                visited_records += 1;
                visitor(row_id);
            }
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.visit_row_ids(plan, |row_id| {
                visited_records += 1;
                visitor(row_id);
            })?;
        }
    }

    Ok(visited_records)
}

fn execute_index_row_id_visitor_strategy_excluding_tombstones<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            visited_records +=
                visit_visible_row_ids_in_batch(index.batch(), plan, tombstones, &mut visitor);
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.visit_row_ids_excluding_tombstones(plan, tombstones, |row_id| {
                visited_records += 1;
                visitor(row_id);
            })?;
        }
    }

    Ok(visited_records)
}

fn execute_index_row_strategy(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
    estimated_candidate_records: usize,
) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => Ok(evaluate_typed_query_plan_rows_with_capacity(
            index.batch(),
            plan,
            estimated_candidate_records,
        )),
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.query_rows(plan)
        }
    }
}

fn execute_index_row_strategy_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => {
            Ok(visible_rows_from_batch(index.batch(), plan, tombstones))
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.query_rows_excluding_tombstones(plan, tombstones)
        }
    }
}

fn execute_index_row_visitor_strategy<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            for row in evaluate_typed_query_plan_rows(index.batch(), plan) {
                visited_records += 1;
                visitor(row.row_id(), row.record());
            }
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.visit_rows(plan, |row_id, record| {
                visited_records += 1;
                visitor(row_id, record);
            })?;
        }
    }

    Ok(visited_records)
}

fn execute_index_row_visitor_strategy_excluding_tombstones<F>(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            visited_records +=
                visit_visible_rows_in_batch(index.batch(), plan, tombstones, &mut visitor);
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.visit_rows_excluding_tombstones(plan, tombstones, |row_id, record| {
                visited_records += 1;
                visitor(row_id, record);
            })?;
        }
    }

    Ok(visited_records)
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

fn execute_index_count_strategy_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<usize, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(0),
        TypedQueryExecutionStrategy::FlatScan => Ok(count_visible_matches_in_batch(
            index.batch(),
            plan,
            tombstones,
        )),
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.count_matches_excluding_tombstones(plan, tombstones)
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

fn execute_index_existence_strategy_excluding_tombstones(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<bool, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(false),
        TypedQueryExecutionStrategy::FlatScan => {
            Ok(has_visible_match_in_batch(index.batch(), plan, tombstones))
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            index.has_match_excluding_tombstones(plan, tombstones)
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

fn execute_append_delta_strategy_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<Vec<RowId>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => {
            let mut row_ids = visible_row_ids_from_batch(view.base().batch(), plan, tombstones);
            row_ids.extend(visible_row_ids_from_batch(
                view.appended(),
                plan,
                tombstones,
            ));
            Ok(row_ids)
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.query_row_ids_excluding_tombstones(plan, tombstones)
        }
    }
}

fn execute_append_delta_row_id_visitor_strategy<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            for row_id in evaluate_typed_query_plan(view.base().batch(), plan) {
                visited_records += 1;
                visitor(row_id);
            }

            for row_id in evaluate_typed_query_plan(view.appended(), plan) {
                visited_records += 1;
                visitor(row_id);
            }
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.visit_row_ids(plan, |row_id| {
                visited_records += 1;
                visitor(row_id);
            })?;
        }
    }

    Ok(visited_records)
}

fn execute_append_delta_row_id_visitor_strategy_excluding_tombstones<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            visited_records +=
                visit_visible_row_ids_in_batch(view.base().batch(), plan, tombstones, &mut visitor);
            visited_records +=
                visit_visible_row_ids_in_batch(view.appended(), plan, tombstones, &mut visitor);
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.visit_row_ids_excluding_tombstones(plan, tombstones, |row_id| {
                visited_records += 1;
                visitor(row_id);
            })?;
        }
    }

    Ok(visited_records)
}

fn execute_append_delta_row_strategy(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => {
            let mut rows = evaluate_typed_query_plan_rows(view.base().batch(), plan);
            rows.extend(evaluate_typed_query_plan_rows(view.appended(), plan));
            Ok(rows)
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.query_rows(plan)
        }
    }
}

fn execute_append_delta_row_strategy_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(Vec::new()),
        TypedQueryExecutionStrategy::FlatScan => {
            let mut rows = visible_rows_from_batch(view.base().batch(), plan, tombstones);
            rows.extend(visible_rows_from_batch(view.appended(), plan, tombstones));
            Ok(rows)
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.query_rows_excluding_tombstones(plan, tombstones)
        }
    }
}

fn execute_append_delta_row_visitor_strategy<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            for row in evaluate_typed_query_plan_rows(view.base().batch(), plan) {
                visited_records += 1;
                visitor(row.row_id(), row.record());
            }

            for row in evaluate_typed_query_plan_rows(view.appended(), plan) {
                visited_records += 1;
                visitor(row.row_id(), row.record());
            }
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.visit_rows(plan, |row_id, record| {
                visited_records += 1;
                visitor(row_id, record);
            })?;
        }
    }

    Ok(visited_records)
}

fn execute_append_delta_row_visitor_strategy_excluding_tombstones<F>(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
    mut visitor: F,
) -> Result<usize, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    let mut visited_records = 0;

    match strategy {
        TypedQueryExecutionStrategy::NoOp => {}
        TypedQueryExecutionStrategy::FlatScan => {
            visited_records +=
                visit_visible_rows_in_batch(view.base().batch(), plan, tombstones, &mut visitor);
            visited_records +=
                visit_visible_rows_in_batch(view.appended(), plan, tombstones, &mut visitor);
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.visit_rows_excluding_tombstones(plan, tombstones, |row_id, record| {
                visited_records += 1;
                visitor(row_id, record);
            })?;
        }
    }

    Ok(visited_records)
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

fn execute_append_delta_count_strategy_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<usize, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(0),
        TypedQueryExecutionStrategy::FlatScan => {
            Ok(
                count_visible_matches_in_batch(view.base().batch(), plan, tombstones)
                    + count_visible_matches_in_batch(view.appended(), plan, tombstones),
            )
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.count_matches_excluding_tombstones(plan, tombstones)
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

fn execute_append_delta_existence_strategy_excluding_tombstones(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    strategy: TypedQueryExecutionStrategy,
) -> Result<bool, IndexedTypedQueryError> {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => Ok(false),
        TypedQueryExecutionStrategy::FlatScan => {
            Ok(
                has_visible_match_in_batch(view.base().batch(), plan, tombstones)
                    || has_visible_match_in_batch(view.appended(), plan, tombstones),
            )
        }
        TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid => {
            view.has_match_excluding_tombstones(plan, tombstones)
        }
    }
}

fn visible_row_ids_from_batch(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Vec<RowId> {
    evaluate_typed_query_plan(batch, plan)
        .into_iter()
        .filter(|row_id| !tombstones.contains(*row_id))
        .collect()
}

fn visible_rows_from_batch(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Vec<TypedQueryResultRow> {
    evaluate_typed_query_plan_rows(batch, plan)
        .into_iter()
        .filter(|row| !tombstones.contains(row.row_id()))
        .collect()
}

fn count_visible_matches_in_batch(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> usize {
    if plan.is_unsatisfiable() {
        return 0;
    }

    batch
        .row_ids()
        .iter()
        .zip(batch.records())
        .filter(|(row_id, record)| {
            !tombstones.contains(**row_id) && record_matches_plan(record, plan)
        })
        .count()
}

fn has_visible_match_in_batch(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> bool {
    if plan.is_unsatisfiable() {
        return false;
    }

    batch
        .row_ids()
        .iter()
        .zip(batch.records())
        .any(|(row_id, record)| !tombstones.contains(*row_id) && record_matches_plan(record, plan))
}

fn visit_visible_row_ids_in_batch<F>(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    visitor: &mut F,
) -> usize
where
    F: FnMut(RowId),
{
    if plan.is_unsatisfiable() {
        return 0;
    }

    let mut visited_records = 0;

    for (row_id, record) in batch.row_ids().iter().zip(batch.records()) {
        if !tombstones.contains(*row_id) && record_matches_plan(record, plan) {
            visited_records += 1;
            visitor(*row_id);
        }
    }

    visited_records
}

fn visit_visible_rows_in_batch<F>(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    visitor: &mut F,
) -> usize
where
    F: FnMut(RowId, &FSERecord),
{
    if plan.is_unsatisfiable() {
        return 0;
    }

    let mut visited_records = 0;

    for (row_id, record) in batch.row_ids().iter().zip(batch.records()) {
        if !tombstones.contains(*row_id) && record_matches_plan(record, plan) {
            visited_records += 1;
            visitor(*row_id, record);
        }
    }

    visited_records
}
