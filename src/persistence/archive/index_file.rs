//! FSE index archive save and load APIs.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::persistence::FSEArchiveSections;
use crate::storage::FSEIndex;

use super::{
    FSEArchiveFileError, FSEArchiveSnapshotError, FSEIndexArchiveSnapshot,
    read_archive_snapshot_file, write_archive_snapshot_file,
};

/// Error returned when saving or loading an FSE index archive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEIndexArchiveError {
    /// Building or reconstructing an archive snapshot failed.
    Snapshot(FSEArchiveSnapshotError),

    /// Archive file access failed.
    File(FSEArchiveFileError),
}

impl fmt::Display for FSEIndexArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::File(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSEIndexArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::File(error) => Some(error),
        }
    }
}

impl From<FSEArchiveSnapshotError> for FSEIndexArchiveError {
    fn from(error: FSEArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<FSEArchiveFileError> for FSEIndexArchiveError {
    fn from(error: FSEArchiveFileError) -> Self {
        Self::File(error)
    }
}

/// Saves an FSE index to a `.fse` archive file.
pub fn save_index_archive_file<P>(path: P, index: &FSEIndex) -> Result<(), FSEIndexArchiveError>
where
    P: AsRef<Path>,
{
    save_index_archive_file_with_sections(path, index, FSEArchiveSections::empty())
}

/// Saves an FSE index to a `.fse` archive file with explicit section metadata.
pub fn save_index_archive_file_with_sections<P>(
    path: P,
    index: &FSEIndex,
    sections: FSEArchiveSections,
) -> Result<(), FSEIndexArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = FSEIndexArchiveSnapshot::from_index_with_sections(index, sections)?;
    write_archive_snapshot_file(path, &snapshot)?;

    Ok(())
}

/// Loads an FSE index from a `.fse` archive file.
pub fn load_index_archive_file<P>(path: P) -> Result<FSEIndex, FSEIndexArchiveError>
where
    P: AsRef<Path>,
{
    let snapshot = read_archive_snapshot_file(path)?;

    snapshot.to_index().map_err(FSEIndexArchiveError::Snapshot)
}
