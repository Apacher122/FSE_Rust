//! Typed query index compaction.

use std::error::Error;
use std::fmt;

use crate::build::FSEBuilder;
use crate::data::{FSERecordBatch, FSERecordBatchError};
use crate::encoding::FSERecordEncoder;
use crate::query::{TypedQueryIndex, TypedQueryIndexBuildError, TypedRowTombstoneSet};

use super::tombstoned::FSETombstonedTypedQueryIndex;

/// Error returned when typed query index compaction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexCompactionError {
    /// Compaction would remove every record in the source index.
    EmptyRetainedRecordSet {
        /// Number of records in the source typed query index.
        base_record_count: usize,

        /// Number of tombstones provided to the compaction operation.
        tombstone_count: usize,
    },

    /// Record batch reconstruction failed before index rebuild.
    RecordBatch(FSERecordBatchError),

    /// Rebuilding the compacted typed query index failed.
    Rebuild(TypedQueryIndexBuildError),
}

impl fmt::Display for FSETypedQueryIndexCompactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRetainedRecordSet { .. } => {
                formatter.write_str("typed query index compaction retained no records")
            }
            Self::RecordBatch(error) => error.fmt(formatter),
            Self::Rebuild(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETypedQueryIndexCompactionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EmptyRetainedRecordSet { .. } => None,
            Self::RecordBatch(error) => Some(error),
            Self::Rebuild(error) => Some(error),
        }
    }
}

impl From<FSERecordBatchError> for FSETypedQueryIndexCompactionError {
    fn from(error: FSERecordBatchError) -> Self {
        Self::RecordBatch(error)
    }
}

impl From<TypedQueryIndexBuildError> for FSETypedQueryIndexCompactionError {
    fn from(error: TypedQueryIndexBuildError) -> Self {
        Self::Rebuild(error)
    }
}

/// Result returned after compacting a tombstoned typed query index.
#[derive(Clone, Debug, PartialEq)]
pub struct FSETypedQueryIndexCompactionResult {
    /// Number of records in the source typed query index.
    pub base_record_count: usize,

    /// Number of tombstones provided to the compaction operation.
    pub tombstone_count: usize,

    /// Number of source records removed by compaction.
    pub removed_record_count: usize,

    /// Number of source records retained by compaction.
    pub retained_record_count: usize,

    /// Compacted typed query index.
    pub query_index: TypedQueryIndex,
}

/// Rebuilds a typed query index after removing tombstoned rows.
pub fn compact_tombstoned_typed_query_index(
    tombstoned: &FSETombstonedTypedQueryIndex,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<FSETypedQueryIndexCompactionResult, FSETypedQueryIndexCompactionError> {
    let base_batch = tombstoned.query_index().batch();
    let base_record_count = base_batch.len();
    let tombstone_count = tombstoned.tombstones().len();
    let (batch, removed_record_count) = compact_record_batch(base_batch, tombstoned.tombstones())?;

    if batch.is_empty() {
        return Err(FSETypedQueryIndexCompactionError::EmptyRetainedRecordSet {
            base_record_count,
            tombstone_count,
        });
    }

    let retained_record_count = batch.len();
    let query_index = TypedQueryIndex::try_build(batch, encoder, builder)?;

    Ok(FSETypedQueryIndexCompactionResult {
        base_record_count,
        tombstone_count,
        removed_record_count,
        retained_record_count,
        query_index,
    })
}

impl FSETombstonedTypedQueryIndex {
    /// Compacts this tombstoned typed query index.
    pub fn compact(
        &self,
        encoder: &impl FSERecordEncoder,
        builder: &FSEBuilder,
    ) -> Result<FSETypedQueryIndexCompactionResult, FSETypedQueryIndexCompactionError> {
        compact_tombstoned_typed_query_index(self, encoder, builder)
    }
}

fn compact_record_batch(
    batch: &FSERecordBatch,
    tombstones: &TypedRowTombstoneSet,
) -> Result<(FSERecordBatch, usize), FSETypedQueryIndexCompactionError> {
    let mut row_ids = Vec::with_capacity(batch.len());
    let mut records = Vec::with_capacity(batch.len());
    let mut removed_record_count = 0;

    for (row_id, record) in batch.row_ids().iter().copied().zip(batch.records()) {
        if tombstones.contains(row_id) {
            removed_record_count += 1;
            continue;
        }

        row_ids.push(row_id);
        records.push(record.clone());
    }

    let batch = FSERecordBatch::try_new(batch.schema().clone(), row_ids, records)?;

    Ok((batch, removed_record_count))
}
