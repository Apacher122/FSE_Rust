//! Archive snapshots for typed row tombstones.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::data::RowId;

/// Error returned when typed row tombstone archive validation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedRowTombstoneArchiveSnapshotError {
    /// A row identifier appears more than once in the tombstone snapshot.
    DuplicateRowId {
        /// Duplicate row identifier.
        row_id: u64,
    },
}

impl fmt::Display for FSETypedRowTombstoneArchiveSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRowId { .. } => {
                formatter.write_str("typed row tombstone archive row identifiers must be unique")
            }
        }
    }
}

impl Error for FSETypedRowTombstoneArchiveSnapshotError {}

/// Serializable snapshot of typed rows marked as deleted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FSETypedRowTombstoneArchiveSnapshot {
    /// Deleted row identifiers.
    pub deleted_row_ids: Vec<u64>,
}

impl FSETypedRowTombstoneArchiveSnapshot {
    /// Creates a tombstone snapshot from row identifiers.
    pub fn from_row_ids<I>(row_ids: I) -> Result<Self, FSETypedRowTombstoneArchiveSnapshotError>
    where
        I: IntoIterator<Item = RowId>,
    {
        let snapshot = Self {
            deleted_row_ids: row_ids.into_iter().map(RowId::value).collect(),
        };
        snapshot.validate()?;

        Ok(snapshot)
    }

    /// Validates the tombstone snapshot.
    pub fn validate(&self) -> Result<(), FSETypedRowTombstoneArchiveSnapshotError> {
        let mut row_ids = self.deleted_row_ids.clone();
        row_ids.sort_unstable();

        for pair in row_ids.windows(2) {
            if pair[0] == pair[1] {
                return Err(FSETypedRowTombstoneArchiveSnapshotError::DuplicateRowId {
                    row_id: pair[0],
                });
            }
        }

        Ok(())
    }

    /// Rebuilds runtime row identifiers from the tombstone snapshot.
    pub fn to_row_ids(&self) -> Result<Vec<RowId>, FSETypedRowTombstoneArchiveSnapshotError> {
        self.validate()?;

        Ok(self
            .deleted_row_ids
            .iter()
            .copied()
            .map(RowId::new)
            .collect())
    }

    /// Returns whether the snapshot contains a row identifier.
    pub fn contains(&self, row_id: RowId) -> bool {
        self.deleted_row_ids.contains(&row_id.value())
    }

    /// Returns the number of tombstones in the snapshot.
    pub fn len(&self) -> usize {
        self.deleted_row_ids.len()
    }

    /// Returns whether the snapshot contains no tombstones.
    pub fn is_empty(&self) -> bool {
        self.deleted_row_ids.is_empty()
    }
}
