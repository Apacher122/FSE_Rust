//! Filesystem access for typed query index archive snapshots.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::persistence::{FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveFileOperation};
use crate::query::TypedQueryIndex;

use super::{
    FSETypedQueryIndexArchiveCodecError, FSETypedQueryIndexArchiveSnapshot,
    FSETypedQueryIndexArchiveSnapshotError, decode_typed_query_index_archive_snapshot,
    encode_typed_query_index_archive_snapshot,
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
            Self::InvalidFileExtension { .. } | Self::Io { .. } => None,
        }
    }
}

impl From<FSETypedQueryIndexArchiveCodecError> for FSETypedQueryIndexArchiveFileError {
    fn from(error: FSETypedQueryIndexArchiveCodecError) -> Self {
        Self::Codec(error)
    }
}

/// Error returned when saving or loading a typed query index archive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveError {
    /// Building or reconstructing a typed query index archive snapshot failed.
    Snapshot(FSETypedQueryIndexArchiveSnapshotError),

    /// Typed query index archive file access failed.
    File(FSETypedQueryIndexArchiveFileError),
}

impl fmt::Display for FSETypedQueryIndexArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::File(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETypedQueryIndexArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::File(error) => Some(error),
        }
    }
}

impl From<FSETypedQueryIndexArchiveSnapshotError> for FSETypedQueryIndexArchiveError {
    fn from(error: FSETypedQueryIndexArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<FSETypedQueryIndexArchiveFileError> for FSETypedQueryIndexArchiveError {
    fn from(error: FSETypedQueryIndexArchiveFileError) -> Self {
        Self::File(error)
    }
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

    let bytes = encode_typed_query_index_archive_snapshot(snapshot)?;
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

    decode_typed_query_index_archive_snapshot(&bytes)
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
