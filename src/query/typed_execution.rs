//! Typed query plan evaluation over record batches.
//!
//! This module evaluates typed query plans against FSE-native record batches
//! and returns matching row identifiers.

use crate::data::{FSERecord, FSERecordBatch, RowId};

use super::{TypedQueryPlan, evaluate_typed_predicate};

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
/// Returns row identifiers for records that satisfy the validated predicate
/// stored in the plan. Row identifiers are returned in batch order.
pub fn evaluate_typed_query_plan(batch: &FSERecordBatch, plan: &TypedQueryPlan) -> Vec<RowId> {
    let mut matches = Vec::new();

    for (row_id, record) in batch.row_ids().iter().zip(batch.records()) {
        if evaluate_typed_predicate(record, plan.predicate()) {
            matches.push(*row_id);
        }
    }
    matches
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
    let mut matches = Vec::new();

    for (row_id, record) in batch.row_ids().iter().zip(batch.records()) {
        if evaluate_typed_predicate(record, plan.predicate()) {
            matches.push(TypedQueryResultRow::new(*row_id, record.clone()));
        }
    }

    matches
}
