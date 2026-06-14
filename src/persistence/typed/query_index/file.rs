//! Filesystem access for typed query index archive snapshots.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::build::FSEBuilder;
use crate::data::FSERecordBatch;
use crate::encoding::FSERecordEncoder;
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveAppendOperationMetadata,
    FSEArchiveAppendOperationMetadataError, FSEArchiveCompactionOperationMetadata,
    FSEArchiveCompactionOperationMetadataError, FSEArchiveFileOperation,
    FSEArchivePayloadHeaderError, FSEArchivePayloadKind, FSEArchiveRebuildPlanMetadata,
    FSEArchiveRebuildPlanMetadataError, decode_archive_payload, encode_archive_payload,
};
use crate::query::{TypedQueryIndex, TypedQueryIndexAppendError, TypedQueryIndexBuildError};

use super::super::tombstone::{
    FSETypedRowTombstoneArchiveError, save_typed_row_tombstone_archive_file,
};
use super::{
    FSETombstonedTypedQueryIndex, FSETombstonedTypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveCodecError, FSETypedQueryIndexArchiveSnapshot,
    FSETypedQueryIndexArchiveSnapshotError, FSETypedQueryIndexCompactionError,
    FSETypedQueryIndexCompactionResult, compact_tombstoned_typed_query_index,
    decode_typed_query_index_archive_snapshot, encode_typed_query_index_archive_snapshot,
    load_typed_query_index_archive_with_tombstones,
};

/// Error returned when typed query index archive file access fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveFileError {
    /// The path does not use the `.fse` archive extension.
    InvalidFileExtension {
        /// Path provided by the caller.
        path: PathBuf,
    },

    /// Archive byte encoding or decoding failed.
    Codec(FSETypedQueryIndexArchiveCodecError),

    /// Archive payload metadata validation failed.
    Payload(FSEArchivePayloadHeaderError),

    /// A filesystem operation failed.
    Io {
        /// Operation that failed.
        operation: FSEArchiveFileOperation,

        /// Path used by the operation.
        path: PathBuf,

        /// Operating-system error kind.
        kind: io::ErrorKind,
    },
}

impl fmt::Display for FSETypedQueryIndexArchiveFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileExtension { .. } => {
                formatter.write_str("typed query index archive path must use the .fse extension")
            }
            Self::Codec(error) => error.fmt(formatter),
            Self::Payload(error) => error.fmt(formatter),
            Self::Io { operation, .. } => match operation {
                FSEArchiveFileOperation::Read => {
                    formatter.write_str("failed to read typed query index archive file")
                }
                FSEArchiveFileOperation::Write => {
                    formatter.write_str("failed to write typed query index archive file")
                }
            },
        }
    }
}

impl Error for FSETypedQueryIndexArchiveFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::Payload(error) => Some(error),
            Self::InvalidFileExtension { .. } | Self::Io { .. } => None,
        }
    }
}

impl From<FSETypedQueryIndexArchiveCodecError> for FSETypedQueryIndexArchiveFileError {
    fn from(error: FSETypedQueryIndexArchiveCodecError) -> Self {
        Self::Codec(error)
    }
}

impl From<FSEArchivePayloadHeaderError> for FSETypedQueryIndexArchiveFileError {
    fn from(error: FSEArchivePayloadHeaderError) -> Self {
        Self::Payload(error)
    }
}

/// Error returned when building, saving, or loading a typed query index archive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveError {
    /// Building or reconstructing a typed query index archive snapshot failed.
    Snapshot(FSETypedQueryIndexArchiveSnapshotError),

    /// Building a typed query index failed.
    Build(TypedQueryIndexBuildError),

    /// Appending records to a typed query index failed.
    Append(TypedQueryIndexAppendError),

    /// Append operation metadata validation failed.
    AppendMetadata(FSEArchiveAppendOperationMetadataError),

    /// Archive rebuild plan metadata validation failed.
    RebuildPlan(FSEArchiveRebuildPlanMetadataError),

    /// Typed query index archive file access failed.
    File(FSETypedQueryIndexArchiveFileError),
}

impl fmt::Display for FSETypedQueryIndexArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::Build(error) => error.fmt(formatter),
            Self::Append(error) => error.fmt(formatter),
            Self::AppendMetadata(error) => error.fmt(formatter),
            Self::RebuildPlan(error) => error.fmt(formatter),
            Self::File(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETypedQueryIndexArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::Build(error) => Some(error),
            Self::Append(error) => Some(error),
            Self::AppendMetadata(error) => Some(error),
            Self::RebuildPlan(error) => Some(error),
            Self::File(error) => Some(error),
        }
    }
}

impl From<FSETypedQueryIndexArchiveSnapshotError> for FSETypedQueryIndexArchiveError {
    fn from(error: FSETypedQueryIndexArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<TypedQueryIndexBuildError> for FSETypedQueryIndexArchiveError {
    fn from(error: TypedQueryIndexBuildError) -> Self {
        Self::Build(error)
    }
}

impl From<TypedQueryIndexAppendError> for FSETypedQueryIndexArchiveError {
    fn from(error: TypedQueryIndexAppendError) -> Self {
        Self::Append(error)
    }
}

impl From<FSEArchiveAppendOperationMetadataError> for FSETypedQueryIndexArchiveError {
    fn from(error: FSEArchiveAppendOperationMetadataError) -> Self {
        Self::AppendMetadata(error)
    }
}

impl From<FSEArchiveRebuildPlanMetadataError> for FSETypedQueryIndexArchiveError {
    fn from(error: FSEArchiveRebuildPlanMetadataError) -> Self {
        Self::RebuildPlan(error)
    }
}

impl From<FSETypedQueryIndexArchiveFileError> for FSETypedQueryIndexArchiveError {
    fn from(error: FSETypedQueryIndexArchiveFileError) -> Self {
        Self::File(error)
    }
}

/// Result returned after appending a typed query index archive.
#[derive(Clone, Debug, PartialEq)]
pub struct FSETypedQueryIndexArchiveAppendResult {
    /// Append operation metadata for the archive update.
    pub append_metadata: FSEArchiveAppendOperationMetadata,

    /// Rebuild plan metadata used by the archive update.
    pub rebuild_plan: FSEArchiveRebuildPlanMetadata,

    /// Rebuilt typed query index saved to the archive.
    pub query_index: TypedQueryIndex,
}

/// Error returned when compacting a typed query index archive file fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveCompactionError {
    /// Loading the typed query index archive with tombstones failed.
    Load(FSETombstonedTypedQueryIndexArchiveError),

    /// Typed query index compaction failed.
    Compaction(FSETypedQueryIndexCompactionError),

    /// Compaction operation metadata validation failed.
    CompactionMetadata(FSEArchiveCompactionOperationMetadataError),

    /// Archive rebuild plan metadata validation failed.
    RebuildPlan(FSEArchiveRebuildPlanMetadataError),

    /// Saving the compacted typed query index archive failed.
    SaveIndex(FSETypedQueryIndexArchiveError),

    /// Saving the cleared typed row tombstone archive failed.
    SaveTombstones(FSETypedRowTombstoneArchiveError),
}

impl fmt::Display for FSETypedQueryIndexArchiveCompactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => error.fmt(formatter),
            Self::Compaction(error) => error.fmt(formatter),
            Self::CompactionMetadata(error) => error.fmt(formatter),
            Self::RebuildPlan(error) => error.fmt(formatter),
            Self::SaveIndex(error) => error.fmt(formatter),
            Self::SaveTombstones(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETypedQueryIndexArchiveCompactionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Load(error) => Some(error),
            Self::Compaction(error) => Some(error),
            Self::CompactionMetadata(error) => Some(error),
            Self::RebuildPlan(error) => Some(error),
            Self::SaveIndex(error) => Some(error),
            Self::SaveTombstones(error) => Some(error),
        }
    }
}

impl From<FSEArchiveCompactionOperationMetadataError> for FSETypedQueryIndexArchiveCompactionError {
    fn from(error: FSEArchiveCompactionOperationMetadataError) -> Self {
        Self::CompactionMetadata(error)
    }
}

impl From<FSEArchiveRebuildPlanMetadataError> for FSETypedQueryIndexArchiveCompactionError {
    fn from(error: FSEArchiveRebuildPlanMetadataError) -> Self {
        Self::RebuildPlan(error)
    }
}

/// Result returned after compacting a typed query index archive file.
#[derive(Clone, Debug, PartialEq)]
pub struct FSETypedQueryIndexArchiveCompactionResult {
    /// In-memory typed query index compaction result.
    pub compaction: FSETypedQueryIndexCompactionResult,

    /// Compaction operation metadata for the archive update.
    pub compaction_metadata: FSEArchiveCompactionOperationMetadata,

    /// Rebuild plan metadata used by the archive update.
    pub rebuild_plan: FSEArchiveRebuildPlanMetadata,

    /// Number of tombstones removed from the tombstone archive.
    pub cleared_tombstone_count: usize,

    /// Number of tombstones remaining after archive compaction.
    pub remaining_tombstone_count: usize,
}

/// Writes a typed query index archive snapshot to a `.fse` file.
pub fn write_typed_query_index_archive_snapshot_file<P>(
    path: P,
    snapshot: &FSETypedQueryIndexArchiveSnapshot,
) -> Result<(), FSETypedQueryIndexArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let payload = encode_typed_query_index_archive_snapshot(snapshot)?;
    let bytes = encode_archive_payload(FSEArchivePayloadKind::TypedQueryIndex, &payload);
    fs::write(path, bytes).map_err(|error| FSETypedQueryIndexArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Write,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

/// Reads a typed query index archive snapshot from a `.fse` file.
pub fn read_typed_query_index_archive_snapshot_file<P>(
    path: P,
) -> Result<FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let bytes = fs::read(path).map_err(|error| FSETypedQueryIndexArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Read,
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;

    let payload = decode_archive_payload(FSEArchivePayloadKind::TypedQueryIndex, &bytes)?;

    decode_typed_query_index_archive_snapshot(&payload)
        .map_err(FSETypedQueryIndexArchiveFileError::Codec)
}

/// Saves a typed query index to a `.fse` archive file.
pub fn save_typed_query_index_archive_file<P>(
    path: P,
    index: &TypedQueryIndex,
) -> Result<(), FSETypedQueryIndexArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(index)?;
    write_typed_query_index_archive_snapshot_file(path, &snapshot)?;

    Ok(())
}

/// Builds and saves a typed query index to a `.fse` archive file.
pub fn build_typed_query_index_archive_file<P>(
    path: P,
    batch: FSERecordBatch,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<TypedQueryIndex, FSETypedQueryIndexArchiveError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    validate_archive_file_extension(path)?;

    let query_index = TypedQueryIndex::try_build(batch, encoder, builder)?;

    save_typed_query_index_archive_file(path, &query_index)?;

    Ok(query_index)
}

/// Loads a typed query index from a `.fse` archive file.
pub fn load_typed_query_index_archive_file<P>(
    path: P,
) -> Result<TypedQueryIndex, FSETypedQueryIndexArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = read_typed_query_index_archive_snapshot_file(path)?;

    snapshot
        .to_typed_query_index()
        .map_err(FSETypedQueryIndexArchiveError::Snapshot)
}

/// Appends records to a typed query index `.fse` archive file.
pub fn append_typed_query_index_archive_file<P>(
    path: P,
    appended: &FSERecordBatch,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<FSETypedQueryIndexArchiveAppendResult, FSETypedQueryIndexArchiveError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let base = load_typed_query_index_archive_file(path)?;

    append_loaded_typed_query_index_archive_file(path, &base, appended, encoder, builder)
}

/// Compacts a typed query index `.fse` archive file and clears its tombstone archive.
pub fn compact_typed_query_index_archive_file<P, Q>(
    query_index_path: P,
    tombstone_path: Q,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<FSETypedQueryIndexArchiveCompactionResult, FSETypedQueryIndexArchiveCompactionError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_index_path = query_index_path.as_ref();
    let tombstone_path = tombstone_path.as_ref();
    let tombstoned =
        load_typed_query_index_archive_with_tombstones(query_index_path, tombstone_path)
            .map_err(FSETypedQueryIndexArchiveCompactionError::Load)?;

    compact_loaded_typed_query_index_archive_file(
        query_index_path,
        tombstone_path,
        tombstoned,
        encoder,
        builder,
    )
}

fn append_loaded_typed_query_index_archive_file(
    path: &Path,
    base: &TypedQueryIndex,
    appended: &FSERecordBatch,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<FSETypedQueryIndexArchiveAppendResult, FSETypedQueryIndexArchiveError> {
    let query_index = base.try_append(appended, encoder, builder)?;
    let append_metadata = FSEArchiveAppendOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        base.batch().len() as u64,
        appended.len() as u64,
    )?;
    let rebuild_plan = FSEArchiveRebuildPlanMetadata::for_append(append_metadata)?;

    save_typed_query_index_archive_file(path, &query_index)?;

    Ok(FSETypedQueryIndexArchiveAppendResult {
        append_metadata,
        rebuild_plan,
        query_index,
    })
}

fn compact_loaded_typed_query_index_archive_file(
    query_index_path: &Path,
    tombstone_path: &Path,
    tombstoned: FSETombstonedTypedQueryIndex,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<FSETypedQueryIndexArchiveCompactionResult, FSETypedQueryIndexArchiveCompactionError> {
    let cleared_tombstone_count = tombstoned.tombstones().len();
    let compaction = compact_tombstoned_typed_query_index(&tombstoned, encoder, builder)
        .map_err(FSETypedQueryIndexArchiveCompactionError::Compaction)?;
    let compaction_metadata = FSEArchiveCompactionOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        compaction.base_record_count as u64,
        compaction.tombstone_count as u64,
        compaction.removed_record_count as u64,
    )?;
    let rebuild_plan = FSEArchiveRebuildPlanMetadata::for_compaction(compaction_metadata)?;

    save_typed_query_index_archive_file(query_index_path, &compaction.query_index)
        .map_err(FSETypedQueryIndexArchiveCompactionError::SaveIndex)?;
    save_typed_row_tombstone_archive_file(tombstone_path, &[])
        .map_err(FSETypedQueryIndexArchiveCompactionError::SaveTombstones)?;

    Ok(FSETypedQueryIndexArchiveCompactionResult {
        compaction,
        compaction_metadata,
        rebuild_plan,
        cleared_tombstone_count,
        remaining_tombstone_count: 0,
    })
}

impl FSETypedQueryIndexArchiveSnapshot {
    /// Writes this typed query index snapshot to a `.fse` file.
    pub fn write_to_archive_file<P>(
        &self,
        path: P,
    ) -> Result<(), FSETypedQueryIndexArchiveFileError>
    where
        P: AsRef<Path>,
    {
        write_typed_query_index_archive_snapshot_file(path, self)
    }

    /// Reads a typed query index snapshot from a `.fse` file.
    pub fn read_from_archive_file<P>(path: P) -> Result<Self, FSETypedQueryIndexArchiveFileError>
    where
        P: AsRef<Path>,
    {
        read_typed_query_index_archive_snapshot_file(path)
    }
}

fn validate_archive_file_extension(path: &Path) -> Result<(), FSETypedQueryIndexArchiveFileError> {
    let expected_extension = FSE_ARCHIVE_FILE_EXTENSION.trim_start_matches('.');
    let actual_extension = path.extension().and_then(|extension| extension.to_str());

    if actual_extension == Some(expected_extension) {
        return Ok(());
    }

    Err(FSETypedQueryIndexArchiveFileError::InvalidFileExtension {
        path: path.to_path_buf(),
    })
}
