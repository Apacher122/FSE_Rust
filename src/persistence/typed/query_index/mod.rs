//! Typed query index archive metadata.

mod codec;
mod file;
mod snapshot;
mod tombstoned;

pub use codec::{
    FSETypedQueryIndexArchiveCodecError, decode_typed_query_index_archive_snapshot,
    encode_typed_query_index_archive_snapshot,
};
pub use file::{
    FSETypedQueryIndexArchiveAppendResult, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFileError, append_typed_query_index_archive_file,
    load_typed_query_index_archive_file, read_typed_query_index_archive_snapshot_file,
    save_typed_query_index_archive_file, write_typed_query_index_archive_snapshot_file,
};
pub use snapshot::{FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError};
pub use tombstoned::{
    FSETombstonedTypedQueryIndex, FSETombstonedTypedQueryIndexArchiveError,
    load_typed_query_index_archive_with_tombstones,
};
