//! Logical footprint reporting for typed query index archives.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::persistence::{FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveFileOperation};

/// Archive component included in a typed query index logical footprint report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveFootprintComponent {
    /// Typed query index archive file.
    QueryIndex,

    /// Typed row tombstone archive file.
    Tombstones,
}

impl FSETypedQueryIndexArchiveFootprintComponent {
    fn name(self) -> &'static str {
        match self {
            Self::QueryIndex => "typed query index archive",
            Self::Tombstones => "typed row tombstone archive",
        }
    }
}

/// Error returned when typed query index archive footprint reporting fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveFootprintError {
    /// The path does not use the `.fse` archive extension.
    InvalidFileExtension {
        /// Archive component associated with the path.
        component: FSETypedQueryIndexArchiveFootprintComponent,

        /// Path provided by the caller.
        path: PathBuf,
    },

    /// A filesystem operation failed.
    Io {
        /// Archive component associated with the operation.
        component: FSETypedQueryIndexArchiveFootprintComponent,

        /// Operation that failed.
        operation: FSEArchiveFileOperation,

        /// Path used by the operation.
        path: PathBuf,

        /// Operating-system error kind.
        kind: io::ErrorKind,
    },

    /// The logical archive byte total exceeded `u64::MAX`.
    TotalArchiveByteCountOverflow {
        /// Bytes in the typed query index archive.
        query_index_archive_bytes: u64,

        /// Bytes in the typed row tombstone archive.
        tombstone_archive_bytes: u64,
    },
}

impl fmt::Display for FSETypedQueryIndexArchiveFootprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileExtension { component, .. } => write!(
                formatter,
                "{} footprint path must use the .fse extension",
                component.name()
            ),
            Self::Io {
                component,
                operation,
                ..
            } => match operation {
                FSEArchiveFileOperation::Read => {
                    write!(
                        formatter,
                        "failed to read {} footprint file",
                        component.name()
                    )
                }
                FSEArchiveFileOperation::Write => write!(
                    formatter,
                    "failed to write {} footprint file",
                    component.name()
                ),
            },
            Self::TotalArchiveByteCountOverflow { .. } => {
                formatter.write_str("typed query index logical archive footprint overflowed")
            }
        }
    }
}

impl Error for FSETypedQueryIndexArchiveFootprintError {}

/// Byte footprint for a logical typed query index archive.
///
/// The report counts the typed query index archive and the active tombstone
/// archive as one logical archive footprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSETypedQueryIndexArchiveFootprint {
    /// Bytes in the typed query index archive file.
    pub query_index_archive_bytes: u64,

    /// Bytes in the typed row tombstone archive file.
    pub tombstone_archive_bytes: u64,

    /// Combined bytes for all required archive components.
    pub total_archive_bytes: u64,
}

impl FSETypedQueryIndexArchiveFootprint {
    /// Creates a logical archive footprint report.
    pub fn try_new(
        query_index_archive_bytes: u64,
        tombstone_archive_bytes: u64,
    ) -> Result<Self, FSETypedQueryIndexArchiveFootprintError> {
        let total_archive_bytes = query_index_archive_bytes
            .checked_add(tombstone_archive_bytes)
            .ok_or(
                FSETypedQueryIndexArchiveFootprintError::TotalArchiveByteCountOverflow {
                    query_index_archive_bytes,
                    tombstone_archive_bytes,
                },
            )?;

        Ok(Self {
            query_index_archive_bytes,
            tombstone_archive_bytes,
            total_archive_bytes,
        })
    }

    /// Returns true when the footprint includes a tombstone archive.
    pub fn includes_tombstone_archive(&self) -> bool {
        self.tombstone_archive_bytes > 0
    }
}

/// Reports the file footprint for a typed query index archive.
pub fn typed_query_index_archive_footprint<P>(
    query_index_path: P,
) -> Result<FSETypedQueryIndexArchiveFootprint, FSETypedQueryIndexArchiveFootprintError>
where
    P: AsRef<Path>,
{
    let query_index_archive_bytes = archive_file_len(
        query_index_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
    )?;

    FSETypedQueryIndexArchiveFootprint::try_new(query_index_archive_bytes, 0)
}

/// Reports the logical file footprint for a typed query index archive with tombstones.
pub fn typed_query_index_archive_with_tombstones_footprint<P, Q>(
    query_index_path: P,
    tombstone_path: Q,
) -> Result<FSETypedQueryIndexArchiveFootprint, FSETypedQueryIndexArchiveFootprintError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_index_archive_bytes = archive_file_len(
        query_index_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
    )?;
    let tombstone_archive_bytes = archive_file_len(
        tombstone_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::Tombstones,
    )?;

    FSETypedQueryIndexArchiveFootprint::try_new(query_index_archive_bytes, tombstone_archive_bytes)
}

fn archive_file_len(
    path: &Path,
    component: FSETypedQueryIndexArchiveFootprintComponent,
) -> Result<u64, FSETypedQueryIndexArchiveFootprintError> {
    validate_archive_file_extension(path, component)?;

    let metadata =
        fs::metadata(path).map_err(|error| FSETypedQueryIndexArchiveFootprintError::Io {
            component,
            operation: FSEArchiveFileOperation::Read,
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;

    Ok(metadata.len())
}

fn validate_archive_file_extension(
    path: &Path,
    component: FSETypedQueryIndexArchiveFootprintComponent,
) -> Result<(), FSETypedQueryIndexArchiveFootprintError> {
    let expected_extension = FSE_ARCHIVE_FILE_EXTENSION.trim_start_matches('.');
    let actual_extension = path.extension().and_then(|extension| extension.to_str());

    if actual_extension == Some(expected_extension) {
        return Ok(());
    }

    Err(
        FSETypedQueryIndexArchiveFootprintError::InvalidFileExtension {
            component,
            path: path.to_path_buf(),
        },
    )
}
