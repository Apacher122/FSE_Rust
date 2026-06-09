//! Filesystem access for typed record batch archive snapshots.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::data::FSERecordBatch;
use crate::persistence::{FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveFileOperation};

use super::{
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveCodecError,
    FSETypedRecordBatchArchiveSnapshotError, decode_typed_record_batch_archive_snapshot,
    encode_typed_record_batch_archive_snapshot,
};

/// Error returned when typed record batch archive file access fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERecordBatchArchiveFileError {
    /// The path does not use the `.fse` archive extension.
    InvalidFileExtension {
        /// Path provided by the caller.
        path: PathBuf,
    },

    /// Archive byte encoding or decoding failed.
    Codec(FSETypedRecordBatchArchiveCodecError),

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

impl fmt::Display for FSERecordBatchArchiveFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileExtension { .. } => {
                formatter.write_str("typed record batch archive path must use the .fse extension")
            }
            Self::Codec(error) => error.fmt(formatter),
            Self::Io { operation, .. } => match operation {
                FSEArchiveFileOperation::Read => {
                    formatter.write_str("failed to read typed record batch archive file")
                }
                FSEArchiveFileOperation::Write => {
                    formatter.write_str("failed to write typed record batch archive file")
                }
            },
        }
    }
}

impl Error for FSERecordBatchArchiveFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::InvalidFileExtension { .. } | Self::Io { .. } => None,
        }
    }
}

impl From<FSETypedRecordBatchArchiveCodecError> for FSERecordBatchArchiveFileError {
    fn from(error: FSETypedRecordBatchArchiveCodecError) -> Self {
        Self::Codec(error)
    }
}

/// Error returned when saving or loading a typed record batch archive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERecordBatchArchiveError {
    /// Building or reconstructing a typed record batch archive snapshot failed.
    Snapshot(FSETypedRecordBatchArchiveSnapshotError),

    /// Typed record batch archive file access failed.
    File(FSERecordBatchArchiveFileError),
}

impl fmt::Display for FSERecordBatchArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::File(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSERecordBatchArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::File(error) => Some(error),
        }
    }
}

impl From<FSETypedRecordBatchArchiveSnapshotError> for FSERecordBatchArchiveError {
    fn from(error: FSETypedRecordBatchArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<FSERecordBatchArchiveFileError> for FSERecordBatchArchiveError {
    fn from(error: FSERecordBatchArchiveFileError) -> Self {
        Self::File(error)
    }
}

/// Writes a typed record batch archive snapshot to a `.fse` file.
pub fn write_typed_record_batch_archive_snapshot_file<P>(
    path: P,
    snapshot: &FSERecordBatchArchiveSnapshot,
) -> Result<(), FSERecordBatchArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let bytes = encode_typed_record_batch_archive_snapshot(snapshot)?;
    fs::write(path, bytes).map_err(|error| FSERecordBatchArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Write,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

/// Reads a typed record batch archive snapshot from a `.fse` file.
pub fn read_typed_record_batch_archive_snapshot_file<P>(
    path: P,
) -> Result<FSERecordBatchArchiveSnapshot, FSERecordBatchArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let bytes = fs::read(path).map_err(|error| FSERecordBatchArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Read,
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;

    decode_typed_record_batch_archive_snapshot(&bytes)
        .map_err(FSERecordBatchArchiveFileError::Codec)
}

/// Saves a typed record batch to a `.fse` archive file.
pub fn save_typed_record_batch_archive_file<P>(
    path: P,
    batch: &FSERecordBatch,
) -> Result<(), FSERecordBatchArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(batch);
    write_typed_record_batch_archive_snapshot_file(path, &snapshot)?;

    Ok(())
}

/// Loads a typed record batch from a `.fse` archive file.
pub fn load_typed_record_batch_archive_file<P>(
    path: P,
) -> Result<FSERecordBatch, FSERecordBatchArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = read_typed_record_batch_archive_snapshot_file(path)?;

    snapshot
        .to_record_batch()
        .map_err(FSERecordBatchArchiveError::Snapshot)
}

impl FSERecordBatchArchiveSnapshot {
    /// Writes this typed record batch snapshot to a `.fse` file.
    pub fn write_to_archive_file<P>(&self, path: P) -> Result<(), FSERecordBatchArchiveFileError>
    where
        P: AsRef<Path>,
    {
        write_typed_record_batch_archive_snapshot_file(path, self)
    }

    /// Reads a typed record batch snapshot from a `.fse` file.
    pub fn read_from_archive_file<P>(path: P) -> Result<Self, FSERecordBatchArchiveFileError>
    where
        P: AsRef<Path>,
    {
        read_typed_record_batch_archive_snapshot_file(path)
    }
}

fn validate_archive_file_extension(path: &Path) -> Result<(), FSERecordBatchArchiveFileError> {
    let expected_extension = FSE_ARCHIVE_FILE_EXTENSION.trim_start_matches('.');
    let actual_extension = path.extension().and_then(|extension| extension.to_str());

    if actual_extension == Some(expected_extension) {
        return Ok(());
    }

    Err(FSERecordBatchArchiveFileError::InvalidFileExtension {
        path: path.to_path_buf(),
    })
}
