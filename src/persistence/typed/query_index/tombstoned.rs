//! Typed query index archive loading with row tombstones.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::data::{FSERecord, RowId};
use crate::query::{
    IndexedTypedQueryError, IndexedTypedQueryReport, IndexedTypedQueryRowReport, QueryCountReport,
    QueryExecutionStats, QueryExistenceReport, TypedQueryIndex, TypedQueryPlan,
    TypedQueryResultRow, TypedRowTombstoneSet,
};

use super::super::tombstone::{
    FSETypedRowTombstoneArchiveError, load_typed_row_tombstone_archive_file,
};
use super::{FSETypedQueryIndexArchiveError, load_typed_query_index_archive_file};

/// Error returned when loading a tombstoned typed query index archive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETombstonedTypedQueryIndexArchiveError {
    /// Loading the typed query index archive failed.
    QueryIndex(FSETypedQueryIndexArchiveError),

    /// Loading the typed row tombstone archive failed.
    Tombstones(FSETypedRowTombstoneArchiveError),
}

impl fmt::Display for FSETombstonedTypedQueryIndexArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryIndex(error) => error.fmt(formatter),
            Self::Tombstones(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETombstonedTypedQueryIndexArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::QueryIndex(error) => Some(error),
            Self::Tombstones(error) => Some(error),
        }
    }
}

impl From<FSETypedQueryIndexArchiveError> for FSETombstonedTypedQueryIndexArchiveError {
    fn from(error: FSETypedQueryIndexArchiveError) -> Self {
        Self::QueryIndex(error)
    }
}

impl From<FSETypedRowTombstoneArchiveError> for FSETombstonedTypedQueryIndexArchiveError {
    fn from(error: FSETypedRowTombstoneArchiveError) -> Self {
        Self::Tombstones(error)
    }
}

/// Typed query index paired with a row tombstone set.
///
/// # Runtime Role
///
/// `FSETombstonedTypedQueryIndex` applies persisted typed row tombstones to the
/// typed query index read path. The stored `TypedQueryIndex` remains available
/// for archive inspection and rebuild workflows.
#[derive(Clone, Debug, PartialEq)]
pub struct FSETombstonedTypedQueryIndex {
    query_index: TypedQueryIndex,
    tombstones: TypedRowTombstoneSet,
}

impl FSETombstonedTypedQueryIndex {
    /// Creates a tombstoned typed query index from existing parts.
    pub fn new(query_index: TypedQueryIndex, tombstones: TypedRowTombstoneSet) -> Self {
        Self {
            query_index,
            tombstones,
        }
    }

    /// Creates a tombstoned typed query index from row identifiers.
    pub fn from_row_ids<I>(query_index: TypedQueryIndex, row_ids: I) -> Self
    where
        I: IntoIterator<Item = RowId>,
    {
        Self::new(query_index, TypedRowTombstoneSet::from_row_ids(row_ids))
    }

    /// Returns the typed query index.
    pub fn query_index(&self) -> &TypedQueryIndex {
        &self.query_index
    }

    /// Returns the typed row tombstones.
    pub fn tombstones(&self) -> &TypedRowTombstoneSet {
        &self.tombstones
    }

    /// Evaluates a typed query plan and returns matching row identifiers.
    pub fn query_row_ids(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        self.query_index
            .query_row_ids_excluding_tombstones(plan, &self.tombstones)
    }

    /// Evaluates a typed query plan and returns row identifiers with statistics.
    pub fn query_row_ids_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
        self.query_index
            .query_row_ids_with_stats_excluding_tombstones(plan, &self.tombstones)
    }

    /// Evaluates a typed query plan and returns matching typed rows.
    pub fn query_rows(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        self.query_index
            .query_rows_excluding_tombstones(plan, &self.tombstones)
    }

    /// Evaluates a typed query plan and returns typed rows with statistics.
    pub fn query_rows_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
        self.query_index
            .query_rows_with_stats_excluding_tombstones(plan, &self.tombstones)
    }

    /// Counts records that satisfy a typed query plan.
    pub fn count_matches(&self, plan: &TypedQueryPlan) -> Result<usize, IndexedTypedQueryError> {
        self.query_index
            .count_matches_excluding_tombstones(plan, &self.tombstones)
    }

    /// Counts records that satisfy a typed query plan and returns statistics.
    pub fn count_matches_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryCountReport, IndexedTypedQueryError> {
        self.query_index
            .count_matches_with_stats_excluding_tombstones(plan, &self.tombstones)
    }

    /// Returns true when a typed query plan matches at least one record.
    pub fn has_match(&self, plan: &TypedQueryPlan) -> Result<bool, IndexedTypedQueryError> {
        self.query_index
            .has_match_excluding_tombstones(plan, &self.tombstones)
    }

    /// Returns typed existence with execution statistics.
    pub fn has_match_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
        self.query_index
            .has_match_with_stats_excluding_tombstones(plan, &self.tombstones)
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
        self.query_index
            .visit_row_ids_excluding_tombstones(plan, &self.tombstones, visitor)
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
        self.query_index
            .visit_rows_excluding_tombstones(plan, &self.tombstones, visitor)
    }
}

/// Loads a typed query index archive and a typed row tombstone archive.
pub fn load_typed_query_index_archive_with_tombstones<P, Q>(
    query_index_path: P,
    tombstone_path: Q,
) -> Result<FSETombstonedTypedQueryIndex, FSETombstonedTypedQueryIndexArchiveError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_index = load_typed_query_index_archive_file(query_index_path)?;
    let row_ids = load_typed_row_tombstone_archive_file(tombstone_path)?;

    Ok(FSETombstonedTypedQueryIndex::from_row_ids(
        query_index,
        row_ids,
    ))
}
