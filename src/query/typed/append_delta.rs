//! Typed query view over a base index and pending appended records.
//!
//! This module supports query execution while appended records are waiting for
//! the next archive maintenance rebuild.

use crate::data::{FSERecord, FSERecordBatch, FSERecordBatchError, RowId};

use super::execution::{
    IndexedTypedQueryError, TypedQueryResultRow, count_typed_query_matches,
    evaluate_typed_query_plan, evaluate_typed_query_plan_rows, typed_query_has_match,
};
use super::index::TypedQueryIndex;
use super::plan::TypedQueryPlan;

/// Borrowed query view over an indexed base batch and an appended record batch.
///
/// # Result Order
///
/// Query methods return base-index matches first, followed by appended batch
/// matches in appended batch order.
#[derive(Clone, Debug)]
pub struct TypedAppendDeltaQueryView<'a> {
    base: &'a TypedQueryIndex,
    appended: &'a FSERecordBatch,
}

impl<'a> TypedAppendDeltaQueryView<'a> {
    /// Creates a typed append delta query view.
    pub fn try_new(
        base: &'a TypedQueryIndex,
        appended: &'a FSERecordBatch,
    ) -> Result<Self, FSERecordBatchError> {
        validate_append_delta(base.batch(), appended)?;

        Ok(Self { base, appended })
    }

    /// Returns the indexed base query data.
    pub fn base(&self) -> &TypedQueryIndex {
        self.base
    }

    /// Returns the appended query data.
    pub fn appended(&self) -> &FSERecordBatch {
        self.appended
    }

    /// Evaluates a typed query plan and returns matching row identifiers.
    pub fn query_row_ids(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        let mut row_ids = self.base.query_row_ids(plan)?;
        row_ids.extend(evaluate_typed_query_plan(self.appended, plan));

        Ok(row_ids)
    }

    /// Evaluates a typed query plan and returns matching typed rows.
    pub fn query_rows(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        let mut rows = self.base.query_rows(plan)?;
        rows.extend(evaluate_typed_query_plan_rows(self.appended, plan));

        Ok(rows)
    }

    /// Counts records that satisfy a typed query plan.
    pub fn count_matches(&self, plan: &TypedQueryPlan) -> Result<usize, IndexedTypedQueryError> {
        Ok(self.base.count_matches(plan)? + count_typed_query_matches(self.appended, plan))
    }

    /// Returns true when a typed query plan matches at least one record.
    pub fn has_match(&self, plan: &TypedQueryPlan) -> Result<bool, IndexedTypedQueryError> {
        if self.base.has_match(plan)? {
            return Ok(true);
        }

        Ok(typed_query_has_match(self.appended, plan))
    }

    /// Visits matching row identifiers for a typed query plan.
    pub fn visit_row_ids<F>(
        &self,
        plan: &TypedQueryPlan,
        mut visitor: F,
    ) -> Result<(), IndexedTypedQueryError>
    where
        F: FnMut(RowId),
    {
        self.base.visit_row_ids(plan, |row_id| {
            visitor(row_id);
        })?;

        for row_id in evaluate_typed_query_plan(self.appended, plan) {
            visitor(row_id);
        }

        Ok(())
    }

    /// Visits matching typed records for a typed query plan.
    pub fn visit_rows<F>(
        &self,
        plan: &TypedQueryPlan,
        mut visitor: F,
    ) -> Result<(), IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        self.base.visit_rows(plan, |row_id, record| {
            visitor(row_id, record);
        })?;

        for row_id in evaluate_typed_query_plan(self.appended, plan) {
            if let Some(record) = self.appended.record_for_row_id(row_id) {
                visitor(row_id, record);
            }
        }

        Ok(())
    }
}

fn validate_append_delta(
    base: &FSERecordBatch,
    appended: &FSERecordBatch,
) -> Result<(), FSERecordBatchError> {
    if base.schema() != appended.schema() {
        return Err(FSERecordBatchError::SchemaMismatch);
    }

    if appended.is_empty() {
        return Err(FSERecordBatchError::EmptyAppendBatch);
    }

    for row_id in appended.row_ids() {
        if base.row_index_for_row_id(*row_id).is_some() {
            return Err(FSERecordBatchError::DuplicateRowId { row_id: *row_id });
        }
    }

    Ok(())
}
