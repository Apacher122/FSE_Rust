//! Filesystem access for typed row tombstone archive snapshots.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::data::RowId;
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveFileOperation, FSEArchivePayloadHeaderError,
    FSEArchivePayloadKind, decode_archive_payload, encode_archive_payload,
};

use super::{
    FSETypedRowTombstoneArchiveCodecError, FSETypedRowTombstoneArchiveSnapshot,
    FSETypedRowTombstoneArchiveSnapshotError, decode_typed_row_tombstone_archive_snapshot,
    encode_typed_row_tombstone_archive_snapshot,
};

/// Error returned when typed row tombstone archive file access fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedRowTombstoneArchiveFileError {
    /// The path does not use the `.fse` archive extension.
    InvalidFileExtension {
        /// Path provided by the caller.
        path: PathBuf,
    },

    /// Archive byte encoding or decoding failed.
    Codec(FSETypedRowTombstoneArchiveCodecError),

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

impl fmt::Display for FSETypedRowTombstoneArchiveFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileExtension { .. } => {
                formatter.write_str("typed row tombstone archive path must use the .fse extension")
            }
            Self::Codec(error) => error.fmt(formatter),
            Self::Payload(error) => error.fmt(formatter),
            Self::Io { operation, .. } => match operation {
                FSEArchiveFileOperation::Read => {
                    formatter.write_str("failed to read typed row tombstone archive file")
                }
                FSEArchiveFileOperation::Write => {
                    formatter.write_str("failed to write typed row tombstone archive file")
                }
            },
        }
    }
}

impl Error for FSETypedRowTombstoneArchiveFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::Payload(error) => Some(error),
            Self::InvalidFileExtension { .. } | Self::Io { .. } => None,
        }
    }
}

impl From<FSETypedRowTombstoneArchiveCodecError> for FSETypedRowTombstoneArchiveFileError {
    fn from(error: FSETypedRowTombstoneArchiveCodecError) -> Self {
        Self::Codec(error)
    }
}

impl From<FSEArchivePayloadHeaderError> for FSETypedRowTombstoneArchiveFileError {
    fn from(error: FSEArchivePayloadHeaderError) -> Self {
        Self::Payload(error)
    }
}

/// Error returned when saving or loading a typed row tombstone archive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedRowTombstoneArchiveError {
    /// Building or reconstructing a typed row tombstone archive snapshot failed.
    Snapshot(FSETypedRowTombstoneArchiveSnapshotError),

    /// The append operation did not contain any row identifiers.
    EmptyAppend,

    /// The append operation contained a row identifier more than once.
    DuplicateAppendedRowId {
        /// Duplicate row identifier.
        row_id: RowId,
    },

    /// Typed row tombstone archive file access failed.
    File(FSETypedRowTombstoneArchiveFileError),
}

impl fmt::Display for FSETypedRowTombstoneArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::EmptyAppend => {
                formatter.write_str("typed row tombstone archive append requires row identifiers")
            }
            Self::DuplicateAppendedRowId { .. } => formatter
                .write_str("typed row tombstone archive append row identifiers must be unique"),
            Self::File(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETypedRowTombstoneArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::EmptyAppend | Self::DuplicateAppendedRowId { .. } => None,
            Self::File(error) => Some(error),
        }
    }
}

impl From<FSETypedRowTombstoneArchiveSnapshotError> for FSETypedRowTombstoneArchiveError {
    fn from(error: FSETypedRowTombstoneArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<FSETypedRowTombstoneArchiveFileError> for FSETypedRowTombstoneArchiveError {
    fn from(error: FSETypedRowTombstoneArchiveFileError) -> Self {
        Self::File(error)
    }
}

/// Result returned after appending typed row tombstones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FSETypedRowTombstoneArchiveAppendResult {
    /// Number of tombstones stored before the append.
    pub base_tombstone_count: usize,

    /// Number of row identifiers provided by the append request.
    pub requested_tombstone_count: usize,

    /// Number of row identifiers added by the append request.
    pub appended_tombstone_count: usize,

    /// Number of tombstones stored after the append.
    pub resulting_tombstone_count: usize,

    /// Tombstones saved to the archive after the append.
    pub row_ids: Vec<RowId>,
}

/// Writes a typed row tombstone archive snapshot to a `.fse` file.
pub fn write_typed_row_tombstone_archive_snapshot_file<P>(
    path: P,
    snapshot: &FSETypedRowTombstoneArchiveSnapshot,
) -> Result<(), FSETypedRowTombstoneArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let payload = encode_typed_row_tombstone_archive_snapshot(snapshot)?;
    let bytes = encode_archive_payload(FSEArchivePayloadKind::TypedRowTombstone, &payload);
    fs::write(path, bytes).map_err(|error| FSETypedRowTombstoneArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Write,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

/// Reads a typed row tombstone archive snapshot from a `.fse` file.
pub fn read_typed_row_tombstone_archive_snapshot_file<P>(
    path: P,
) -> Result<FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let bytes = fs::read(path).map_err(|error| FSETypedRowTombstoneArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Read,
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;

    let payload = decode_archive_payload(FSEArchivePayloadKind::TypedRowTombstone, &bytes)?;

    decode_typed_row_tombstone_archive_snapshot(&payload)
        .map_err(FSETypedRowTombstoneArchiveFileError::Codec)
}

/// Saves typed row tombstones to a `.fse` archive file.
pub fn save_typed_row_tombstone_archive_file<P>(
    path: P,
    row_ids: &[RowId],
) -> Result<(), FSETypedRowTombstoneArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = FSETypedRowTombstoneArchiveSnapshot::from_row_ids(row_ids.iter().copied())?;
    write_typed_row_tombstone_archive_snapshot_file(path, &snapshot)?;

    Ok(())
}

/// Loads typed row tombstones from a `.fse` archive file.
pub fn load_typed_row_tombstone_archive_file<P>(
    path: P,
) -> Result<Vec<RowId>, FSETypedRowTombstoneArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = read_typed_row_tombstone_archive_snapshot_file(path)?;

    snapshot
        .to_row_ids()
        .map_err(FSETypedRowTombstoneArchiveError::Snapshot)
}

/// Appends typed row tombstones to a `.fse` archive file.
pub fn append_typed_row_tombstone_archive_file<P>(
    path: P,
    appended: &[RowId],
) -> Result<FSETypedRowTombstoneArchiveAppendResult, FSETypedRowTombstoneArchiveError>
where
    P: AsRef<Path>,
{
    validate_append_row_ids(appended)?;

    let path = path.as_ref();
    let base = load_typed_row_tombstone_archive_file(path)?;
    let base_tombstone_count = base.len();
    let requested_tombstone_count = appended.len();
    let row_ids = merge_tombstones(base, appended);
    let resulting_tombstone_count = row_ids.len();
    let appended_tombstone_count = resulting_tombstone_count - base_tombstone_count;

    save_typed_row_tombstone_archive_file(path, &row_ids)?;

    Ok(FSETypedRowTombstoneArchiveAppendResult {
        base_tombstone_count,
        requested_tombstone_count,
        appended_tombstone_count,
        resulting_tombstone_count,
        row_ids,
    })
}

impl FSETypedRowTombstoneArchiveSnapshot {
    /// Writes this typed row tombstone snapshot to a `.fse` file.
    pub fn write_to_archive_file<P>(
        &self,
        path: P,
    ) -> Result<(), FSETypedRowTombstoneArchiveFileError>
    where
        P: AsRef<Path>,
    {
        write_typed_row_tombstone_archive_snapshot_file(path, self)
    }

    /// Reads a typed row tombstone snapshot from a `.fse` file.
    pub fn read_from_archive_file<P>(path: P) -> Result<Self, FSETypedRowTombstoneArchiveFileError>
    where
        P: AsRef<Path>,
    {
        read_typed_row_tombstone_archive_snapshot_file(path)
    }
}

fn validate_append_row_ids(row_ids: &[RowId]) -> Result<(), FSETypedRowTombstoneArchiveError> {
    if row_ids.is_empty() {
        return Err(FSETypedRowTombstoneArchiveError::EmptyAppend);
    }

    let mut sorted = row_ids.to_vec();
    sorted.sort_unstable();

    for pair in sorted.windows(2) {
        if pair[0] == pair[1] {
            return Err(FSETypedRowTombstoneArchiveError::DuplicateAppendedRowId {
                row_id: pair[0],
            });
        }
    }

    Ok(())
}

fn merge_tombstones(mut base: Vec<RowId>, appended: &[RowId]) -> Vec<RowId> {
    base.extend_from_slice(appended);
    base.sort_unstable();
    base.dedup();
    base
}

fn validate_archive_file_extension(
    path: &Path,
) -> Result<(), FSETypedRowTombstoneArchiveFileError> {
    let expected_extension = FSE_ARCHIVE_FILE_EXTENSION.trim_start_matches('.');
    let actual_extension = path.extension().and_then(|extension| extension.to_str());

    if actual_extension == Some(expected_extension) {
        return Ok(());
    }

    Err(FSETypedRowTombstoneArchiveFileError::InvalidFileExtension {
        path: path.to_path_buf(),
    })
}
