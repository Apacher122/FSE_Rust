//! Typed row tombstone filtering.

use crate::data::{FSERecord, RowId};
use crate::query::execution::{QueryCountReport, QueryExecutionStats, QueryExistenceReport};

use super::execution::{
    IndexedTypedQueryError, IndexedTypedQueryReport, IndexedTypedQueryRowReport,
    TypedQueryResultRow,
};
use super::index::TypedQueryIndex;
use super::plan::TypedQueryPlan;
use super::planned_execution::{
    PlannedTypedQueryCountReport, PlannedTypedQueryExistenceReport, PlannedTypedQueryRowIdReport,
    PlannedTypedQueryRowReport, PlannedTypedQueryVisitReport,
};

/// Runtime set of row identifiers excluded from typed query results.
///
/// # Runtime Role
///
/// `TypedRowTombstoneSet` stores logical row identifiers that should be skipped
/// after indexed query references have been resolved to typed records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypedRowTombstoneSet {
    row_ids: Vec<RowId>,
}

impl TypedRowTombstoneSet {
    /// Creates a tombstone set from row identifiers.
    pub fn from_row_ids<I>(row_ids: I) -> Self
    where
        I: IntoIterator<Item = RowId>,
    {
        let mut row_ids = row_ids.into_iter().collect::<Vec<_>>();
        row_ids.sort_unstable();
        row_ids.dedup();

        Self { row_ids }
    }

    /// Returns true when the row identifier is tombstoned.
    pub fn contains(&self, row_id: RowId) -> bool {
        self.row_ids.binary_search(&row_id).is_ok()
    }

    /// Returns tombstoned row identifiers in sorted order.
    pub fn row_ids(&self) -> &[RowId] {
        &self.row_ids
    }

    /// Returns the number of tombstoned row identifiers.
    pub fn len(&self) -> usize {
        self.row_ids.len()
    }

    /// Returns true when no row identifiers are tombstoned.
    pub fn is_empty(&self) -> bool {
        self.row_ids.is_empty()
    }
}

/// Borrowed typed query index paired with a row tombstone set.
///
/// The view exposes tombstone-aware query methods without taking ownership of
/// the typed query index or the tombstone set.
#[derive(Clone, Copy, Debug)]
pub struct TypedTombstonedQueryIndexView<'a> {
    index: &'a TypedQueryIndex,
    tombstones: &'a TypedRowTombstoneSet,
}

impl<'a> TypedTombstonedQueryIndexView<'a> {
    /// Creates a borrowed tombstoned typed query index view.
    pub fn new(index: &'a TypedQueryIndex, tombstones: &'a TypedRowTombstoneSet) -> Self {
        Self { index, tombstones }
    }

    /// Returns the typed query index.
    pub fn index(&self) -> &'a TypedQueryIndex {
        self.index
    }

    /// Returns the typed row tombstones.
    pub fn tombstones(&self) -> &'a TypedRowTombstoneSet {
        self.tombstones
    }

    /// Evaluates a typed query plan and returns matching row identifiers.
    pub fn query_row_ids(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        self.index
            .query_row_ids_excluding_tombstones(plan, self.tombstones)
    }

    /// Evaluates a typed query plan and returns row identifiers with statistics.
    pub fn query_row_ids_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
        self.index
            .query_row_ids_with_stats_excluding_tombstones(plan, self.tombstones)
    }

    /// Evaluates row-id output using typed query planning diagnostics.
    pub fn query_row_ids_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryRowIdReport, IndexedTypedQueryError> {
        self.index
            .query_row_ids_with_planning_excluding_tombstones(plan, self.tombstones)
    }

    /// Evaluates a typed query plan and returns matching typed rows.
    pub fn query_rows(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        self.index
            .query_rows_excluding_tombstones(plan, self.tombstones)
    }

    /// Evaluates a typed query plan and returns typed rows with statistics.
    pub fn query_rows_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
        self.index
            .query_rows_with_stats_excluding_tombstones(plan, self.tombstones)
    }

    /// Evaluates typed row output using typed query planning diagnostics.
    pub fn query_rows_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryRowReport, IndexedTypedQueryError> {
        self.index
            .query_rows_with_planning_excluding_tombstones(plan, self.tombstones)
    }

    /// Counts records that satisfy a typed query plan.
    pub fn count_matches(&self, plan: &TypedQueryPlan) -> Result<usize, IndexedTypedQueryError> {
        self.index
            .count_matches_excluding_tombstones(plan, self.tombstones)
    }

    /// Counts records that satisfy a typed query plan and returns statistics.
    pub fn count_matches_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryCountReport, IndexedTypedQueryError> {
        self.index
            .count_matches_with_stats_excluding_tombstones(plan, self.tombstones)
    }

    /// Counts records using typed query planning diagnostics.
    pub fn count_matches_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryCountReport, IndexedTypedQueryError> {
        self.index
            .count_matches_with_planning_excluding_tombstones(plan, self.tombstones)
    }

    /// Returns true when a typed query plan matches at least one record.
    pub fn has_match(&self, plan: &TypedQueryPlan) -> Result<bool, IndexedTypedQueryError> {
        self.index
            .has_match_excluding_tombstones(plan, self.tombstones)
    }

    /// Returns existence output with execution statistics.
    pub fn has_match_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
        self.index
            .has_match_with_stats_excluding_tombstones(plan, self.tombstones)
    }

    /// Returns typed existence using typed query planning diagnostics.
    pub fn has_match_with_planning(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<PlannedTypedQueryExistenceReport, IndexedTypedQueryError> {
        self.index
            .has_match_with_planning_excluding_tombstones(plan, self.tombstones)
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
        self.index
            .visit_row_ids_excluding_tombstones(plan, self.tombstones, visitor)
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
        self.index
            .visit_row_ids_with_planning_excluding_tombstones(plan, self.tombstones, visitor)
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
        self.index
            .visit_rows_excluding_tombstones(plan, self.tombstones, visitor)
    }

    /// Visits typed records using typed query planning diagnostics.
    pub fn visit_rows_with_planning<F>(
        &self,
        plan: &TypedQueryPlan,
        visitor: F,
    ) -> Result<PlannedTypedQueryVisitReport, IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        self.index
            .visit_rows_with_planning_excluding_tombstones(plan, self.tombstones, visitor)
    }
}
