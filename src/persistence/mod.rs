//! Persistence metadata for FSE archives.
//!
//! This module defines versioned archive metadata used by durable FSE storage.

mod codec;
mod manifest;
mod snapshot;

pub use codec::{FSEArchiveCodecError, decode_archive_snapshot, encode_archive_snapshot};
pub use manifest::{
    FSE_ARCHIVE_FILE_EXTENSION, FSE_ARCHIVE_FORMAT_VERSION, FSE_ARCHIVE_MAGIC, FSEArchiveManifest,
    FSEArchiveManifestError, FSEArchiveSections,
};
pub use snapshot::{
    FSEArchiveSnapshotError, FSEIndexArchiveSnapshot, FSEPartitionNodeArchiveRecord,
};
