//! Persistence metadata for FSE archives.
//!
//! This module defines versioned archive metadata used by durable FSE storage.

mod manifest;

pub use manifest::{
    FSE_ARCHIVE_FILE_EXTENSION, FSE_ARCHIVE_FORMAT_VERSION, FSE_ARCHIVE_MAGIC, FSEArchiveManifest,
    FSEArchiveManifestError, FSEArchiveSections,
};
