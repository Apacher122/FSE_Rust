//! Typed row tombstone archive metadata.

mod codec;
mod file;
mod snapshot;

pub use codec::{
    FSETypedRowTombstoneArchiveCodecError, decode_typed_row_tombstone_archive_snapshot,
    encode_typed_row_tombstone_archive_snapshot,
};
pub use file::{
    FSETypedRowTombstoneArchiveAppendResult, FSETypedRowTombstoneArchiveError,
    FSETypedRowTombstoneArchiveFileError, append_typed_row_tombstone_archive_file,
    load_typed_row_tombstone_archive_file, read_typed_row_tombstone_archive_snapshot_file,
    save_typed_row_tombstone_archive_file, write_typed_row_tombstone_archive_snapshot_file,
};
pub use snapshot::{FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveSnapshotError};
