//! Filesystem access for FSE archive snapshots.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveCodecError, FSEIndexArchiveSnapshot,
    decode_archive_snapshot, encode_archive_snapshot,
};

/// Filesystem operation performed against an FSE archive file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEArchiveFileOperation {
    /// Read an archive file.
    Read,

    /// Write an archive file.
    Write,
}

/// Error returned when archive file access fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveFileError {
    /// The path does not use the `.fse` archive extension.
    InvalidFileExtension {
        /// Path provided by the caller.
        path: PathBuf,
    },

    /// Archive byte encoding or decoding failed.
    Codec(FSEArchiveCodecError),

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

impl fmt::Display for FSEArchiveFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileExtension { .. } => {
                formatter.write_str("archive path must use the .fse extension")
            }
            Self::Codec(error) => error.fmt(formatter),
            Self::Io { operation, .. } => match operation {
                FSEArchiveFileOperation::Read => {
                    formatter.write_str("failed to read FSE archive file")
                }
                FSEArchiveFileOperation::Write => {
                    formatter.write_str("failed to write FSE archive file")
                }
            },
        }
    }
}

impl Error for FSEArchiveFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::InvalidFileExtension { .. } | Self::Io { .. } => None,
        }
    }
}

impl From<FSEArchiveCodecError> for FSEArchiveFileError {
    fn from(error: FSEArchiveCodecError) -> Self {
        Self::Codec(error)
    }
}

/// Writes an archive snapshot to a `.fse` file.
pub fn write_archive_snapshot_file<P>(
    path: P,
    snapshot: &FSEIndexArchiveSnapshot,
) -> Result<(), FSEArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let bytes = encode_archive_snapshot(snapshot)?;
    fs::write(path, bytes).map_err(|error| FSEArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Write,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

/// Reads an archive snapshot from a `.fse` file.
pub fn read_archive_snapshot_file<P>(
    path: P,
) -> Result<FSEIndexArchiveSnapshot, FSEArchiveFileError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    validate_archive_file_extension(path)?;

    let bytes = fs::read(path).map_err(|error| FSEArchiveFileError::Io {
        operation: FSEArchiveFileOperation::Read,
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;

    decode_archive_snapshot(&bytes).map_err(FSEArchiveFileError::Codec)
}

impl FSEIndexArchiveSnapshot {
    /// Writes this snapshot to a `.fse` file.
    pub fn write_to_archive_file<P>(&self, path: P) -> Result<(), FSEArchiveFileError>
    where
        P: AsRef<Path>,
    {
        write_archive_snapshot_file(path, self)
    }

    /// Reads a snapshot from a `.fse` file.
    pub fn read_from_archive_file<P>(path: P) -> Result<Self, FSEArchiveFileError>
    where
        P: AsRef<Path>,
    {
        read_archive_snapshot_file(path)
    }
}

fn validate_archive_file_extension(path: &Path) -> Result<(), FSEArchiveFileError> {
    let expected_extension = FSE_ARCHIVE_FILE_EXTENSION.trim_start_matches('.');
    let actual_extension = path.extension().and_then(|extension| extension.to_str());

    if actual_extension == Some(expected_extension) {
        return Ok(());
    }

    Err(FSEArchiveFileError::InvalidFileExtension {
        path: path.to_path_buf(),
    })
}
