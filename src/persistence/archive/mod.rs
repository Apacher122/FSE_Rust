//! Numeric FSE index archive components.

mod codec;
mod file;
mod index_file;
mod snapshot;

pub use codec::{FSEArchiveCodecError, decode_archive_snapshot, encode_archive_snapshot};
pub use file::{
    FSEArchiveFileError, FSEArchiveFileOperation, read_archive_snapshot_file,
    write_archive_snapshot_file,
};
pub use index_file::{
    FSEIndexArchiveError, load_index_archive_file, save_index_archive_file,
    save_index_archive_file_with_sections,
};
pub use snapshot::{
    FSEArchiveSnapshotError, FSEIndexArchiveSnapshot, FSEPartitionNodeArchiveRecord,
};
