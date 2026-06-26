//! Typed indexed query interface.

use std::error::Error;
use std::fmt;

use crate::build::{BuildInputError, FSEBuilder, RowMappedFSEIndex};
use crate::data::{FSERecord, FSERecordBatch, FSERecordBatchError, RowId};
use crate::encoding::{FSERecordBatchEncodingError, FSERecordEncoder, encode_record_batch};
use crate::query::execution::{QueryCountReport, QueryExecutionStats, QueryExistenceReport};

use super::execution::{
    IndexedTypedQueryError, IndexedTypedQueryReport, IndexedTypedQueryRowReport,
    TypedQueryResultRow, count_indexed_typed_query_matches,
    count_indexed_typed_query_matches_with_stats, evaluate_indexed_typed_query_plan,
    evaluate_indexed_typed_query_plan_rows,
    evaluate_indexed_typed_query_plan_rows_excluding_tombstones,
    evaluate_indexed_typed_query_plan_rows_with_stats,
    evaluate_indexed_typed_query_plan_rows_with_stats_excluding_tombstones,
    evaluate_indexed_typed_query_plan_with_stats,
    evaluate_indexed_typed_query_plan_with_stats_excluding_tombstones,
    indexed_typed_query_has_match, indexed_typed_query_has_match_excluding_tombstones,
    indexed_typed_query_has_match_with_stats,
    indexed_typed_query_has_match_with_stats_excluding_tombstones,
    visit_indexed_typed_query_row_ids, visit_indexed_typed_query_row_ids_excluding_tombstones,
    visit_indexed_typed_query_rows, visit_indexed_typed_query_rows_excluding_tombstones,
};
use super::plan::TypedQueryPlan;
use super::planned_execution::{
    PlannedTypedQueryCountReport, PlannedTypedQueryExistenceReport, PlannedTypedQueryRowIdReport,
    PlannedTypedQueryRowReport, PlannedTypedQueryVisitReport, planned_typed_query_count_matches,
    planned_typed_query_count_matches_excluding_tombstones, planned_typed_query_has_match,
    planned_typed_query_has_match_excluding_tombstones, planned_typed_query_row_ids,
    planned_typed_query_row_ids_excluding_tombstones, planned_typed_query_rows,
    planned_typed_query_rows_excluding_tombstones, planned_typed_query_visit_row_ids,
    planned_typed_query_visit_row_ids_excluding_tombstones, planned_typed_query_visit_rows,
    planned_typed_query_visit_rows_excluding_tombstones,
};
use super::planning::TypedQueryPlanningStatistics;
use super::tombstone::TypedRowTombstoneSet;

/// Error returned when typed indexed query construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedQueryIndexBuildError {
    /// Record batch encoding failed.
    Encoding(FSERecordBatchEncodingError),

    /// Index construction rejected the encoded batch.
    Build(BuildInputError),
}

impl fmt::Display for TypedQueryIndexBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding(error) => error.fmt(formatter),
            Self::Build(error) => error.fmt(formatter),
        }
    }
}

impl Error for TypedQueryIndexBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encoding(error) => Some(error),
            Self::Build(error) => Some(error),
        }
    }
}

impl From<FSERecordBatchEncodingError> for TypedQueryIndexBuildError {
    fn from(error: FSERecordBatchEncodingError) -> Self {
        Self::Encoding(error)
    }
}

impl From<BuildInputError> for TypedQueryIndexBuildError {
    fn from(error: BuildInputError) -> Self {
        Self::Build(error)
    }
}

/// Error returned when typed indexed query append fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedQueryIndexAppendError {
    /// Record batch append validation failed.
    RecordBatch(FSERecordBatchError),

    /// Rebuilding the typed query index failed.
    Rebuild(TypedQueryIndexBuildError),
}

impl fmt::Display for TypedQueryIndexAppendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordBatch(error) => error.fmt(formatter),
            Self::Rebuild(error) => error.fmt(formatter),
        }
    }
}

impl Error for TypedQueryIndexAppendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RecordBatch(error) => Some(error),
            Self::Rebuild(error) => Some(error),
        }
    }
}

impl From<FSERecordBatchError> for TypedQueryIndexAppendError {
    fn from(error: FSERecordBatchError) -> Self {
        Self::RecordBatch(error)
    }
}

impl From<TypedQueryIndexBuildError> for TypedQueryIndexAppendError {
    fn from(error: TypedQueryIndexBuildError) -> Self {
        Self::Rebuild(error)
    }
}

/// Typed records paired with a row-mapped FSE index.
///
/// # Runtime Role
///
/// `TypedQueryIndex` keeps typed records beside the geometric index that was
/// built from their encoded coordinates. Query execution uses the index for
/// pruning and the record batch for exact typed predicate evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedQueryIndex {
    batch: FSERecordBatch,
    index: RowMappedFSEIndex,
    planning_statistics: TypedQueryPlanningStatistics,
}

impl TypedQueryIndex {
    /// Creates a typed query index from existing parts.
    pub fn from_parts(batch: FSERecordBatch, index: RowMappedFSEIndex) -> Self {
        let planning_statistics = TypedQueryPlanningStatistics::from_batch(&batch);

        Self {
            batch,
            index,
            planning_statistics,
        }
    }

    /// Encodes a record batch and builds a row-mapped index for typed queries.
    pub fn try_build(
        batch: FSERecordBatch,
        encoder: &impl FSERecordEncoder,
        builder: &FSEBuilder,
    ) -> Result<Self, TypedQueryIndexBuildError> {
        let encoded = encode_record_batch(&batch, encoder)?;
        let index = builder.try_build_row_mapped_encoded_batch(&encoded)?;

        Ok(Self::from_parts(batch, index))
    }

    /// Appends records and rebuilds the row-mapped index.
    pub fn try_append(
        &self,
        appended: &FSERecordBatch,
        encoder: &impl FSERecordEncoder,
        builder: &FSEBuilder,
    ) -> Result<Self, TypedQueryIndexAppendError> {
        let batch = self.batch.try_append(appended)?;

        Self::try_build(batch, encoder, builder).map_err(Into::into)
    }

    /// Returns the typed record batch.
    pub fn batch(&self) -> &FSERecordBatch {
        &self.batch
    }

    /// Returns the row-mapped FSE index.
    pub fn index(&self) -> &RowMappedFSEIndex {
        &self.index
    }

    pub(super) fn planning_statistics(&self) -> &TypedQueryPlanningStatistics {
        &self.planning_statistics
    }

    /// Evaluates a typed query plan and returns matching row identifiers.
    pub fn query_row_ids(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan(&self.index, &self.batch, plan)
    }

    /// Evaluates a typed query plan and returns row identifiers with statistics.
    pub fn query_row_ids_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_with_stats(&self.index, &self.batch, plan)
    }

    /// Evaluates row-id output using typed query planning diagnostics.
    pub fn query_row_ids_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryRowIdReport, IndexedTypedQueryError> {
        planned_typed_query_row_ids(self, plan)
    }

    /// Evaluates row-id output with tombstone filtering using typed query
    /// planning diagnostics.
    pub fn query_row_ids_with_planning_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<PlannedTypedQueryRowIdReport, IndexedTypedQueryError> {
        planned_typed_query_row_ids_excluding_tombstones(self, plan, tombstones)
    }

    /// Evaluates a typed query plan and excludes tombstoned row identifiers.
    pub fn query_row_ids_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        super::execution::evaluate_indexed_typed_query_plan_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Evaluates a typed query plan with statistics and excludes tombstoned row
    /// identifiers.
    pub fn query_row_ids_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_with_stats_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Evaluates a typed query plan and returns matching typed rows.
    pub fn query_rows(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_rows(&self.index, &self.batch, plan)
    }

    /// Evaluates a typed query plan and returns typed rows with statistics.
    pub fn query_rows_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_rows_with_stats(&self.index, &self.batch, plan)
    }

    /// Evaluates typed row output using typed query planning diagnostics.
    pub fn query_rows_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryRowReport, IndexedTypedQueryError> {
        planned_typed_query_rows(self, plan)
    }

    /// Evaluates typed row output with tombstone filtering using typed query
    /// planning diagnostics.
    pub fn query_rows_with_planning_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<PlannedTypedQueryRowReport, IndexedTypedQueryError> {
        planned_typed_query_rows_excluding_tombstones(self, plan, tombstones)
    }

    /// Evaluates a typed query plan, returns typed rows, and excludes
    /// tombstoned row identifiers.
    pub fn query_rows_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_rows_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Evaluates a typed query plan with row statistics and excludes tombstoned
    /// row identifiers.
    pub fn query_rows_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_rows_with_stats_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Counts records that satisfy a typed query plan.
    pub fn count_matches(&self, plan: &TypedQueryPlan) -> Result<usize, IndexedTypedQueryError> {
        count_indexed_typed_query_matches(&self.index, &self.batch, plan)
    }

    /// Counts records that satisfy a typed query plan and returns statistics.
    pub fn count_matches_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryCountReport, IndexedTypedQueryError> {
        count_indexed_typed_query_matches_with_stats(&self.index, &self.batch, plan)
    }

    /// Counts records using typed query planning diagnostics.
    pub fn count_matches_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryCountReport, IndexedTypedQueryError> {
        planned_typed_query_count_matches(self, plan)
    }

    /// Counts records with tombstone filtering using typed query planning
    /// diagnostics.
    pub fn count_matches_with_planning_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<PlannedTypedQueryCountReport, IndexedTypedQueryError> {
        planned_typed_query_count_matches_excluding_tombstones(self, plan, tombstones)
    }

    /// Counts records that satisfy a typed query plan while excluding
    /// tombstoned row identifiers.
    pub fn count_matches_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<usize, IndexedTypedQueryError> {
        super::execution::count_indexed_typed_query_matches_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Counts records that satisfy a typed query plan with statistics while
    /// excluding tombstoned row identifiers.
    pub fn count_matches_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<QueryCountReport, IndexedTypedQueryError> {
        super::execution::count_indexed_typed_query_matches_with_stats_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Returns true when a typed query plan matches at least one record.
    pub fn has_match(&self, plan: &TypedQueryPlan) -> Result<bool, IndexedTypedQueryError> {
        indexed_typed_query_has_match(&self.index, &self.batch, plan)
    }

    /// Returns typed existence with execution statistics.
    pub fn has_match_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
        indexed_typed_query_has_match_with_stats(&self.index, &self.batch, plan)
    }

    /// Returns typed existence using typed query planning diagnostics.
    pub fn has_match_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryExistenceReport, IndexedTypedQueryError> {
        planned_typed_query_has_match(self, plan)
    }

    /// Returns typed existence with tombstone filtering using typed query
    /// planning diagnostics.
    pub fn has_match_with_planning_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<PlannedTypedQueryExistenceReport, IndexedTypedQueryError> {
        planned_typed_query_has_match_excluding_tombstones(self, plan, tombstones)
    }

    /// Returns true when a typed query plan matches at least one
    /// non-tombstoned record.
    pub fn has_match_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<bool, IndexedTypedQueryError> {
        indexed_typed_query_has_match_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Returns typed existence with execution statistics while excluding
    /// tombstoned row identifiers.
    pub fn has_match_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
        indexed_typed_query_has_match_with_stats_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
        )
    }

    /// Visits matching row identifiers for a typed query plan.
    pub fn visit_row_ids<F>(
        &self,
        plan: &TypedQueryPlan,
        visitor: F,
    ) -> Result<QueryExecutionStats, IndexedTypedQueryError>
    where
        F: FnMut(RowId),
    {
        visit_indexed_typed_query_row_ids(&self.index, &self.batch, plan, visitor)
    }

    /// Visits row identifiers using typed query planning diagnostics.
    pub fn visit_row_ids_with_planning<F>(
        &self,
        plan: &TypedQueryPlan,
        visitor: F,
    ) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
    where
        F: FnMut(RowId),
    {
        planned_typed_query_visit_row_ids(self, plan, visitor)
    }

    /// Visits row identifiers with tombstone filtering using typed query
    /// planning diagnostics.
    pub fn visit_row_ids_with_planning_excluding_tombstones<F>(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
        visitor: F,
    ) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
    where
        F: FnMut(RowId),
    {
        planned_typed_query_visit_row_ids_excluding_tombstones(self, plan, tombstones, visitor)
    }

    /// Visits matching row identifiers for a typed query plan while excluding
    /// tombstoned row identifiers.
    pub fn visit_row_ids_excluding_tombstones<F>(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
        visitor: F,
    ) -> Result<QueryExecutionStats, IndexedTypedQueryError>
    where
        F: FnMut(RowId),
    {
        visit_indexed_typed_query_row_ids_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
            visitor,
        )
    }

    /// Visits matching typed records for a typed query plan.
    pub fn visit_rows<F>(
        &self,
        plan: &TypedQueryPlan,
        visitor: F,
    ) -> Result<QueryExecutionStats, IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        visit_indexed_typed_query_rows(&self.index, &self.batch, plan, visitor)
    }

    /// Visits typed rows using typed query planning diagnostics.
    pub fn visit_rows_with_planning<F>(
        &self,
        plan: &TypedQueryPlan,
        visitor: F,
    ) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        planned_typed_query_visit_rows(self, plan, visitor)
    }

    /// Visits typed rows with tombstone filtering using typed query planning
    /// diagnostics.
    pub fn visit_rows_with_planning_excluding_tombstones<F>(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
        visitor: F,
    ) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        planned_typed_query_visit_rows_excluding_tombstones(self, plan, tombstones, visitor)
    }

    /// Visits matching typed records for a typed query plan while excluding
    /// tombstoned row identifiers.
    pub fn visit_rows_excluding_tombstones<F>(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
        visitor: F,
    ) -> Result<QueryExecutionStats, IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        visit_indexed_typed_query_rows_excluding_tombstones(
            &self.index,
            &self.batch,
            plan,
            tombstones,
            visitor,
        )
    }
}
