//! Archive snapshots for typed query indexes.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::persistence::{
    FSERecordBatchArchiveSnapshot, FSERowMappedArchiveSnapshotError,
    FSERowMappedIndexArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError,
};
use crate::query::TypedQueryIndex;

/// Error returned when typed query index archive snapshot validation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveSnapshotError {
    /// The row-mapped index snapshot is invalid.
    Index(FSERowMappedArchiveSnapshotError),

    /// The typed record batch snapshot is invalid.
    Batch(FSETypedRecordBatchArchiveSnapshotError),

    /// The index and batch snapshots contain different row-id counts.
    RowIdCountMismatch {
        /// Number of row identifiers stored by the row-mapped index snapshot.
        indexed_row_id_count: usize,

        /// Number of row identifiers stored by the typed record batch snapshot.
        batch_row_id_count: usize,
    },

    /// The index and batch snapshots contain different row identifiers.
    RowIdMismatch {
        /// Row identifier found in index row-id order.
        indexed_row_id: u64,

        /// Row identifier found in batch row-id order.
        batch_row_id: u64,
    },
}

impl fmt::Display for FSETypedQueryIndexArchiveSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(error) => error.fmt(formatter),
            Self::Batch(error) => error.fmt(formatter),
            Self::RowIdCountMismatch { .. } => {
                formatter.write_str("typed query index archive row-id counts must match")
            }
            Self::RowIdMismatch { .. } => {
                formatter.write_str("typed query index archive row identifiers must match")
            }
        }
    }
}

impl Error for FSETypedQueryIndexArchiveSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Index(error) => Some(error),
            Self::Batch(error) => Some(error),
            Self::RowIdCountMismatch { .. } | Self::RowIdMismatch { .. } => None,
        }
    }
}

impl From<FSERowMappedArchiveSnapshotError> for FSETypedQueryIndexArchiveSnapshotError {
    fn from(error: FSERowMappedArchiveSnapshotError) -> Self {
        Self::Index(error)
    }
}

impl From<FSETypedRecordBatchArchiveSnapshotError> for FSETypedQueryIndexArchiveSnapshotError {
    fn from(error: FSETypedRecordBatchArchiveSnapshotError) -> Self {
        Self::Batch(error)
    }
}

/// Serializable snapshot of a typed query index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FSETypedQueryIndexArchiveSnapshot {
    /// Row-mapped geometric index snapshot.
    pub index: FSERowMappedIndexArchiveSnapshot,

    /// Typed record batch snapshot.
    pub batch: FSERecordBatchArchiveSnapshot,
}

impl FSETypedQueryIndexArchiveSnapshot {
    /// Creates a typed query index archive snapshot from a runtime typed query index.
    pub fn from_typed_query_index(
        index: &TypedQueryIndex,
    ) -> Result<Self, FSETypedQueryIndexArchiveSnapshotError> {
        let snapshot = Self {
            index: FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(index.index())?,
            batch: FSERecordBatchArchiveSnapshot::from_record_batch(index.batch()),
        };
        snapshot.validate()?;

        Ok(snapshot)
    }

    /// Validates the typed query index archive snapshot.
    pub fn validate(&self) -> Result<(), FSETypedQueryIndexArchiveSnapshotError> {
        self.index.validate()?;
        self.batch.validate()?;
        validate_row_identity(&self.index, &self.batch)
    }

    /// Rebuilds a runtime typed query index from the archive snapshot.
    pub fn to_typed_query_index(
        &self,
    ) -> Result<TypedQueryIndex, FSETypedQueryIndexArchiveSnapshotError> {
        self.validate()?;

        let batch = self.batch.to_record_batch()?;
        let index = self.index.to_row_mapped_index()?;

        Ok(TypedQueryIndex::from_parts(batch, index))
    }
}

fn validate_row_identity(
    index: &FSERowMappedIndexArchiveSnapshot,
    batch: &FSERecordBatchArchiveSnapshot,
) -> Result<(), FSETypedQueryIndexArchiveSnapshotError> {
    let mut indexed_row_ids = index
        .leaf_row_id_records
        .iter()
        .flat_map(|record| record.row_ids.iter().copied())
        .collect::<Vec<_>>();
    let mut batch_row_ids = batch.row_ids.clone();

    indexed_row_ids.sort_unstable();
    batch_row_ids.sort_unstable();

    if indexed_row_ids.len() != batch_row_ids.len() {
        return Err(FSETypedQueryIndexArchiveSnapshotError::RowIdCountMismatch {
            indexed_row_id_count: indexed_row_ids.len(),
            batch_row_id_count: batch_row_ids.len(),
        });
    }

    for (indexed_row_id, batch_row_id) in indexed_row_ids.into_iter().zip(batch_row_ids) {
        if indexed_row_id != batch_row_id {
            return Err(FSETypedQueryIndexArchiveSnapshotError::RowIdMismatch {
                indexed_row_id,
                batch_row_id,
            });
        }
    }

    Ok(())
}
