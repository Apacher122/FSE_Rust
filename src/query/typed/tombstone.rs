//! Typed row tombstone filtering.

use crate::data::RowId;

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
