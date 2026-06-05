//! Typed query plan evaluation over record batches.
//!
//! This module evaluates typed query plans against FSE-native record batches
//! and returns matching row identifiers.

use crate::data::{FSERecordBatch, RowId};

use super::{TypedQueryPlan, evaluate_typed_predicate};

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
