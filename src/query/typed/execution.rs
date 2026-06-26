//! Typed query plan evaluation over record batches.
//!
//! This module evaluates typed query plans against FSE-native record batches
//! and returns matching row identifiers.

use std::error::Error;
use std::fmt;

use crate::build::RowMappedFSEIndex;
use crate::data::{FSERecord, FSERecordBatch, RowId};
use crate::math::Scalar;

use super::super::execution::{
    QueryCountReport, QueryExecutionStats, QueryExistenceReport, QueryResultReference,
    execute_query_references_with_stats,
};
use super::evaluator::evaluate_typed_predicate;
use super::plan::TypedQueryPlan;
use super::tombstone::TypedRowTombstoneSet;

/// Error returned when indexed typed query execution cannot resolve row identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexedTypedQueryError {
    /// A leaf row reference had no row identifier mapping.
    MissingRowMapping {
        /// Leaf node containing the referenced row.
        node_id: usize,

        /// Row index inside the leaf residual block.
        row_index: usize,
    },

    /// A mapped row identifier was not present in the record batch.
    MissingRecord {
        /// Missing row identifier.
        row_id: RowId,
    },
}

impl fmt::Display for IndexedTypedQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRowMapping { node_id, row_index } => write!(
                formatter,
                "indexed query reference {node_id}:{row_index} has no row id mapping"
            ),
            Self::MissingRecord { row_id } => {
                write!(
                    formatter,
                    "record batch does not contain row id {}",
                    row_id.value()
                )
            }
        }
    }
}

impl Error for IndexedTypedQueryError {}

/// Row-id result report for indexed typed query execution.
///
/// # Fields
///
/// `row_ids` contains matching logical row identifiers in index traversal order.
/// `stats` contains geometric query execution statistics.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedTypedQueryReport {
    /// Matching logical row identifiers.
    pub row_ids: Vec<RowId>,

    /// Runtime statistics for the geometric execution path.
    pub stats: QueryExecutionStats,
}

/// Row result report for indexed typed query execution.
///
/// # Fields
///
/// `rows` contains matching typed records in index traversal order. `stats`
/// contains geometric query execution statistics.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedTypedQueryRowReport {
    /// Matching typed query result rows.
    pub rows: Vec<TypedQueryResultRow>,

    /// Runtime statistics for the geometric execution path.
    pub stats: QueryExecutionStats,
}

/// Matching typed query result row.
///
/// # Fields
///
/// `row_id` is the stable logical row identifier. `record` is the typed record
/// stored at that row identifier.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedQueryResultRow {
    row_id: RowId,
    record: FSERecord,
}

impl TypedQueryResultRow {
    /// Creates a typed query result row.
    pub fn new(row_id: RowId, record: FSERecord) -> Self {
        Self { row_id, record }
    }

    /// Returns the stable logical row identifier.
    pub fn row_id(&self) -> RowId {
        self.row_id
    }

    /// Returns the matching typed record.
    pub fn record(&self) -> &FSERecord {
        &self.record
    }
}

/// Evaluates a typed query plan against a record batch.
///
/// # Returns
///
/// Returns row identifiers for records that satisfy the validated predicates
/// stored in the plan. Row identifiers are returned in batch order.
pub fn evaluate_typed_query_plan(batch: &FSERecordBatch, plan: &TypedQueryPlan) -> Vec<RowId> {
    evaluate_typed_query_plan_with_capacity(batch, plan, 0)
}

pub(super) fn evaluate_typed_query_plan_with_capacity(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    expected_matches: usize,
) -> Vec<RowId> {
    if plan.is_unsatisfiable() {
        return Vec::new();
    }

    let mut matches = Vec::with_capacity(expected_matches.min(batch.len()));

    for (row_id, record) in batch.row_ids().iter().zip(batch.records()) {
        if record_matches_plan(record, plan) {
            matches.push(*row_id);
        }
    }
    matches
}

/// Counts records that satisfy a typed query plan.
///
/// # Returns
///
/// Returns the number of batch records accepted by the validated predicates
/// stored in the plan.
pub fn count_typed_query_matches(batch: &FSERecordBatch, plan: &TypedQueryPlan) -> usize {
    if plan.is_unsatisfiable() {
        return 0;
    }

    batch
        .records()
        .iter()
        .filter(|record| record_matches_plan(record, plan))
        .count()
}

/// Returns true when a typed query plan matches at least one record.
pub fn typed_query_has_match(batch: &FSERecordBatch, plan: &TypedQueryPlan) -> bool {
    if plan.is_unsatisfiable() {
        return false;
    }

    batch
        .records()
        .iter()
        .any(|record| record_matches_plan(record, plan))
}

/// Evaluates a typed query plan against a record batch and returns result rows.
///
/// # Returns
///
/// Returns matching row identifiers and typed records in batch order.
pub fn evaluate_typed_query_plan_rows(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Vec<TypedQueryResultRow> {
    evaluate_typed_query_plan_rows_with_capacity(batch, plan, 0)
}

pub(super) fn evaluate_typed_query_plan_rows_with_capacity(
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    expected_matches: usize,
) -> Vec<TypedQueryResultRow> {
    if plan.is_unsatisfiable() {
        return Vec::new();
    }

    let mut matches = Vec::with_capacity(expected_matches.min(batch.len()));

    for (row_id, record) in batch.row_ids().iter().zip(batch.records()) {
        if record_matches_plan(record, plan) {
            matches.push(TypedQueryResultRow::new(*row_id, record.clone()));
        }
    }

    matches
}

/// Evaluates a typed query plan through a row-mapped FSE index.
///
/// # Returns
///
/// Returns matching row identifiers in index traversal order.
pub fn evaluate_indexed_typed_query_plan(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<Vec<RowId>, IndexedTypedQueryError> {
    Ok(evaluate_indexed_typed_query_plan_with_stats(index, batch, plan)?.row_ids)
}

/// Evaluates a typed query plan through a row-mapped FSE index and excludes
/// tombstoned row identifiers.
pub fn evaluate_indexed_typed_query_plan_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<Vec<RowId>, IndexedTypedQueryError> {
    Ok(
        evaluate_indexed_typed_query_plan_with_stats_excluding_tombstones(
            index, batch, plan, tombstones,
        )?
        .row_ids,
    )
}

/// Evaluates a typed query plan through a row-mapped FSE index with statistics.
///
/// # Runtime Role
///
/// The query region in the plan is executed against the FSE hierarchy. Matching
/// leaf row references are resolved to row identifiers and checked against the
/// validated typed predicates stored in the plan.
pub fn evaluate_indexed_typed_query_plan_with_stats(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
    evaluate_indexed_typed_query_plan_with_tombstone_filter(index, batch, plan, None)
}

/// Evaluates a typed query plan through a row-mapped FSE index with statistics
/// and excludes tombstoned row identifiers.
pub fn evaluate_indexed_typed_query_plan_with_stats_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
    evaluate_indexed_typed_query_plan_with_tombstone_filter(index, batch, plan, Some(tombstones))
}

fn evaluate_indexed_typed_query_plan_with_tombstone_filter(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
    if plan.is_unsatisfiable() {
        return Ok(IndexedTypedQueryReport {
            row_ids: Vec::new(),
            stats: unsatisfiable_plan_stats(index),
        });
    }

    let reference_report = execute_query_references_with_stats(index.index(), plan.query_region());
    let mut row_ids = Vec::with_capacity(reference_report.matches.len());

    for reference in reference_report.matches {
        let row_id = row_id_for_reference(index, reference)?;
        if row_is_tombstoned(row_id, tombstones) {
            continue;
        }

        let record = record_for_row_id(batch, row_id)?;

        if record_matches_plan(record, plan) {
            row_ids.push(row_id);
        }
    }

    let mut stats = reference_report.stats;
    stats.matched_records = row_ids.len();

    Ok(IndexedTypedQueryReport { row_ids, stats })
}

/// Counts records that satisfy an indexed typed query plan.
pub fn count_indexed_typed_query_matches(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<usize, IndexedTypedQueryError> {
    Ok(count_indexed_typed_query_matches_with_stats(index, batch, plan)?.matched_records)
}

/// Counts records that satisfy an indexed typed query plan while excluding
/// tombstoned row identifiers.
pub fn count_indexed_typed_query_matches_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<usize, IndexedTypedQueryError> {
    Ok(
        count_indexed_typed_query_matches_with_stats_excluding_tombstones(
            index, batch, plan, tombstones,
        )?
        .matched_records,
    )
}

/// Counts records that satisfy an indexed typed query plan and returns statistics.
///
/// # Runtime Role
///
/// The query region in the plan is evaluated by the FSE hierarchy. Each
/// retained row reference is resolved to a typed record and checked against the
/// validated predicates stored in the plan.
pub fn count_indexed_typed_query_matches_with_stats(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<QueryCountReport, IndexedTypedQueryError> {
    let stats = visit_indexed_typed_query_rows_with_tombstone_filter(
        index,
        batch,
        plan,
        None,
        |_row_id, _record| {},
    )?;

    Ok(QueryCountReport {
        matched_records: stats.matched_records,
        stats,
    })
}

/// Counts records that satisfy an indexed typed query plan with statistics
/// while excluding tombstoned row identifiers.
pub fn count_indexed_typed_query_matches_with_stats_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<QueryCountReport, IndexedTypedQueryError> {
    let stats = visit_indexed_typed_query_rows_with_tombstone_filter(
        index,
        batch,
        plan,
        Some(tombstones),
        |_row_id, _record| {},
    )?;

    Ok(QueryCountReport {
        matched_records: stats.matched_records,
        stats,
    })
}

/// Returns true when an indexed typed query plan matches at least one record.
pub fn indexed_typed_query_has_match(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<bool, IndexedTypedQueryError> {
    Ok(indexed_typed_query_has_match_with_stats(index, batch, plan)?.has_match)
}

/// Returns true when an indexed typed query plan matches at least one
/// non-tombstoned record.
pub fn indexed_typed_query_has_match_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<bool, IndexedTypedQueryError> {
    Ok(
        indexed_typed_query_has_match_with_stats_excluding_tombstones(
            index, batch, plan, tombstones,
        )?
        .has_match,
    )
}

/// Returns indexed typed existence with execution statistics.
///
/// # Runtime Role
///
/// The report records the boolean result and the number of typed candidate
/// records inspected before the result was established.
pub fn indexed_typed_query_has_match_with_stats(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
    indexed_typed_query_has_match_with_tombstone_filter(index, batch, plan, None)
}

/// Returns indexed typed existence with execution statistics while excluding
/// tombstoned row identifiers.
pub fn indexed_typed_query_has_match_with_stats_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
    indexed_typed_query_has_match_with_tombstone_filter(index, batch, plan, Some(tombstones))
}

fn indexed_typed_query_has_match_with_tombstone_filter(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
    if plan.is_unsatisfiable() {
        return Ok(QueryExistenceReport {
            has_match: false,
            inspected_records: 0,
            stats: unsatisfiable_plan_stats(index),
        });
    }

    let reference_report = execute_query_references_with_stats(index.index(), plan.query_region());
    let mut inspected_records = 0;
    let mut has_match = false;

    for reference in reference_report.matches {
        inspected_records += 1;

        let row_id = row_id_for_reference(index, reference)?;
        if row_is_tombstoned(row_id, tombstones) {
            continue;
        }

        let record = record_for_row_id(batch, row_id)?;

        if record_matches_plan(record, plan) {
            has_match = true;
            break;
        }
    }

    let mut stats = reference_report.stats;
    stats.reconstructed_records = inspected_records;
    stats.matched_records = usize::from(has_match);
    stats.candidate_ratio = candidate_ratio(inspected_records, stats.total_records);

    Ok(QueryExistenceReport {
        has_match,
        inspected_records,
        stats,
    })
}

/// Visits matching row identifiers for an indexed typed query plan.
///
/// # Runtime Role
///
/// The visitor is called once for each typed record accepted by the validated
/// predicates stored in the plan.
pub fn visit_indexed_typed_query_row_ids<F>(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    mut visitor: F,
) -> Result<QueryExecutionStats, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    visit_indexed_typed_query_rows(index, batch, plan, |row_id, _record| {
        visitor(row_id);
    })
}

/// Visits matching row identifiers for an indexed typed query plan while
/// excluding tombstoned row identifiers.
pub fn visit_indexed_typed_query_row_ids_excluding_tombstones<F>(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    mut visitor: F,
) -> Result<QueryExecutionStats, IndexedTypedQueryError>
where
    F: FnMut(RowId),
{
    visit_indexed_typed_query_rows_with_tombstone_filter(
        index,
        batch,
        plan,
        Some(tombstones),
        |row_id, _record| {
            visitor(row_id);
        },
    )
}

/// Visits matching typed records for an indexed typed query plan.
///
/// # Runtime Role
///
/// The visitor receives the logical row identifier and the record stored in the
/// indexed batch for each accepted typed match.
pub fn visit_indexed_typed_query_rows<F>(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    mut visitor: F,
) -> Result<QueryExecutionStats, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    visit_indexed_typed_query_rows_with_tombstone_filter(
        index,
        batch,
        plan,
        None,
        |row_id, record| {
            visitor(row_id, record);
        },
    )
}

/// Visits matching typed records for an indexed typed query plan while excluding
/// tombstoned row identifiers.
pub fn visit_indexed_typed_query_rows_excluding_tombstones<F>(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
    mut visitor: F,
) -> Result<QueryExecutionStats, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    visit_indexed_typed_query_rows_with_tombstone_filter(
        index,
        batch,
        plan,
        Some(tombstones),
        |row_id, record| {
            visitor(row_id, record);
        },
    )
}

fn visit_indexed_typed_query_rows_with_tombstone_filter<F>(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
    mut visitor: F,
) -> Result<QueryExecutionStats, IndexedTypedQueryError>
where
    F: FnMut(RowId, &FSERecord),
{
    if plan.is_unsatisfiable() {
        return Ok(unsatisfiable_plan_stats(index));
    }

    let reference_report = execute_query_references_with_stats(index.index(), plan.query_region());
    let mut matched_records = 0;

    for reference in reference_report.matches {
        let row_id = row_id_for_reference(index, reference)?;
        if row_is_tombstoned(row_id, tombstones) {
            continue;
        }

        let record = record_for_row_id(batch, row_id)?;

        if record_matches_plan(record, plan) {
            matched_records += 1;
            visitor(row_id, record);
        }
    }

    let mut stats = reference_report.stats;
    stats.matched_records = matched_records;

    Ok(stats)
}

/// Evaluates a typed query plan through a row-mapped FSE index and returns rows.
///
/// # Returns
///
/// Returns matching row identifiers and typed records in index traversal order.
pub fn evaluate_indexed_typed_query_plan_rows(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
    Ok(evaluate_indexed_typed_query_plan_rows_with_stats(index, batch, plan)?.rows)
}

/// Evaluates a typed query plan through a row-mapped FSE index, returns rows,
/// and excludes tombstoned row identifiers.
pub fn evaluate_indexed_typed_query_plan_rows_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
    Ok(
        evaluate_indexed_typed_query_plan_rows_with_stats_excluding_tombstones(
            index, batch, plan, tombstones,
        )?
        .rows,
    )
}

/// Evaluates a typed query plan through a row-mapped FSE index and returns rows
/// with statistics.
pub fn evaluate_indexed_typed_query_plan_rows_with_stats(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
    evaluate_indexed_typed_query_plan_rows_with_tombstone_filter(index, batch, plan, None)
}

/// Evaluates a typed query plan through a row-mapped FSE index, returns rows
/// with statistics, and excludes tombstoned row identifiers.
pub fn evaluate_indexed_typed_query_plan_rows_with_stats_excluding_tombstones(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
    evaluate_indexed_typed_query_plan_rows_with_tombstone_filter(
        index,
        batch,
        plan,
        Some(tombstones),
    )
}

fn evaluate_indexed_typed_query_plan_rows_with_tombstone_filter(
    index: &RowMappedFSEIndex,
    batch: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
    let mut rows = Vec::new();
    let stats = visit_indexed_typed_query_rows_with_tombstone_filter(
        index,
        batch,
        plan,
        tombstones,
        |row_id, record| {
            rows.push(TypedQueryResultRow::new(row_id, record.clone()));
        },
    )?;

    Ok(IndexedTypedQueryRowReport { rows, stats })
}

fn row_id_for_reference(
    index: &RowMappedFSEIndex,
    reference: QueryResultReference,
) -> Result<RowId, IndexedTypedQueryError> {
    index
        .row_id_for_leaf_row(reference.node_id, reference.row_index)
        .ok_or(IndexedTypedQueryError::MissingRowMapping {
            node_id: reference.node_id,
            row_index: reference.row_index,
        })
}

fn record_for_row_id(
    batch: &FSERecordBatch,
    row_id: RowId,
) -> Result<&FSERecord, IndexedTypedQueryError> {
    batch
        .record_for_row_id(row_id)
        .ok_or(IndexedTypedQueryError::MissingRecord { row_id })
}

fn unsatisfiable_plan_stats(index: &RowMappedFSEIndex) -> QueryExecutionStats {
    QueryExecutionStats {
        total_leaves: index.index().leaf_count(),
        total_records: index.index().root_node().cardinality,
        ..QueryExecutionStats::default()
    }
}

pub(super) fn record_matches_plan(record: &FSERecord, plan: &TypedQueryPlan) -> bool {
    plan.predicates()
        .iter()
        .all(|predicate| evaluate_typed_predicate(record, predicate))
}

fn row_is_tombstoned(row_id: RowId, tombstones: Option<&TypedRowTombstoneSet>) -> bool {
    tombstones.is_some_and(|tombstones| tombstones.contains(row_id))
}

fn candidate_ratio(inspected_records: usize, total_records: usize) -> Scalar {
    if total_records == 0 {
        return 0.0;
    }

    inspected_records as Scalar / total_records as Scalar
}
