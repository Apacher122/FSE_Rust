//! Filesystem access for row-mapped FSE archives.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::build::RowMappedFSEIndex;
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveFileOperation, FSEArchivePayloadHeaderError,
    FSEArchivePayloadKind, decode_archive_payload, encode_archive_payload,
};

use super::{
    FSERowMappedArchiveCodecError, FSERowMappedArchiveSnapshotError,
    FSERowMappedIndexArchiveSnapshot, decode_row_mapped_archive_snapshot,
    encode_row_mapped_archive_snapshot,
};

/// Error returned when row-mapped archive file access fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERowMappedArchiveFileError {
    /// The path does not use the `.fse` archive extension.
    InvalidFileExtension {
        /// Path provided by the caller.
        path: PathBuf,
    },

    /// Archive byte encoding or decoding failed.
    Codec(FSERowMappedArchiveCodecError),

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

impl fmt::Display for FSERowMappedArchiveFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileExtension { .. } => {
                formatter.write_str("row-mapped archive path must use the .fse extension")
            }
            Self::Codec(error) => error.fmt(formatter),
            Self::Payload(error) => error.fmt(formatter),
            Self::Io { operation, .. } => match operation {
                FSEArchiveFileOperation::Read => {
                    formatter.write_str("failed to read row-mapped FSE archive file")
                }
                FSEArchiveFileOperation::Write => {
                    formatter.write_str("failed to write row-mapped FSE archive file")
                }
            },
        }
    }
}

impl Error for FSERowMappedArchiveFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::Payload(error) => Some(error),
            Self::InvalidFileExtension { .. } | Self::Io { .. } => None,
        }
    }
}

impl From<FSERowMappedArchiveCodecError> for FSERowMappedArchiveFileError {
    fn from(error: FSERowMappedArchiveCodecError) -> Self {
        Self::Codec(error)
    }
}

impl From<FSEArchivePayloadHeaderError> for FSERowMappedArchiveFileError {
    fn from(error: FSEArchivePayloadHeaderError) -> Self {
        Self::Payload(error)
    }
}

/// Error returned when saving or loading a row-mapped FSE index archive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERowMappedIndexArchiveError {
    /// Building or reconstructing a row-mapped archive snapshot failed.
    Snapshot(FSERowMappedArchiveSnapshotError),

    /// Row-mapped archive file access failed.
    File(FSERowMappedArchiveFileError),
}

impl fmt::Display for FSERowMappedIndexArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::File(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSERowMappedIndexArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::File(error) => Some(error),
        }
    }
}

impl From<FSERowMappedArchiveSnapshotError> for FSERowMappedIndexArchiveError {
    fn from(error: FSERowMappedArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<FSERowMappedArchiveFileError> for FSERowMappedIndexArchiveError {
    fn from(error: FSERowMappedArchiveFileError) -> Self {
        Self::File(error)
    }
}

/// Writes a row-mapped archive snapshot to a `.fse` file.
pub fn write_row_mapped_archive_snapshot_file<P>(
    path: P,
    snapshot: &FSERowMappedIndexArchiveSnapshot,
) -> Result<(), FSERowMappedArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let payload = encode_row_mapped_archive_snapshot(snapshot)?;
    let bytes = encode_archive_payload(FSEArchivePayloadKind::RowMappedIndex, &payload);
    fs::write(path, bytes).map_err(|error| FSERowMappedArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Write,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

/// Reads a row-mapped archive snapshot from a `.fse` file.
pub fn read_row_mapped_archive_snapshot_file<P>(
    path: P,
) -> Result<FSERowMappedIndexArchiveSnapshot, FSERowMappedArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let bytes = fs::read(path).map_err(|error| FSERowMappedArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Read,
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;

    let payload = decode_archive_payload(FSEArchivePayloadKind::RowMappedIndex, &bytes)?;

    decode_row_mapped_archive_snapshot(&payload).map_err(FSERowMappedArchiveFileError::Codec)
}

/// Saves a row-mapped FSE index to a `.fse` archive file.
pub fn save_row_mapped_index_archive_file<P>(
    path: P,
    index: &RowMappedFSEIndex,
) -> Result<(), FSERowMappedIndexArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(index)?;
    write_row_mapped_archive_snapshot_file(path, &snapshot)?;

    Ok(())
}

/// Loads a row-mapped FSE index from a `.fse` archive file.
pub fn load_row_mapped_index_archive_file<P>(
    path: P,
) -> Result<RowMappedFSEIndex, FSERowMappedIndexArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = read_row_mapped_archive_snapshot_file(path)?;

    snapshot
        .to_row_mapped_index()
        .map_err(FSERowMappedIndexArchiveError::Snapshot)
}

impl FSERowMappedIndexArchiveSnapshot {
    /// Writes this row-mapped snapshot to a `.fse` file.
    pub fn write_to_archive_file<P>(&self, path: P) -> Result<(), FSERowMappedArchiveFileError>
    where
        P: AsRef<Path>,
    {
        write_row_mapped_archive_snapshot_file(path, self)
    }

    /// Reads a row-mapped snapshot from a `.fse` file.
    pub fn read_from_archive_file<P>(path: P) -> Result<Self, FSERowMappedArchiveFileError>
    where
        P: AsRef<Path>,
    {
        read_row_mapped_archive_snapshot_file(path)
    }
}

fn validate_archive_file_extension(path: &Path) -> Result<(), FSERowMappedArchiveFileError> {
    let expected_extension = FSE_ARCHIVE_FILE_EXTENSION.trim_start_matches('.');
    let actual_extension = path.extension().and_then(|extension| extension.to_str());

    if actual_extension == Some(expected_extension) {
        return Ok(());
    }

    Err(FSERowMappedArchiveFileError::InvalidFileExtension {
        path: path.to_path_buf(),
    })
}
