//! Typed query index archive metadata.

mod codec;
mod compaction;
mod file;
mod maintenance;
mod snapshot;
mod tombstoned;

pub use codec::{
    FSETypedQueryIndexArchiveCodecError, decode_typed_query_index_archive_snapshot,
    encode_typed_query_index_archive_snapshot,
};
pub use compaction::{
    FSETypedQueryIndexCompactionError, FSETypedQueryIndexCompactionResult,
    compact_tombstoned_typed_query_index,
};
pub use file::{
    FSETypedQueryIndexArchiveAppendResult, FSETypedQueryIndexArchiveCompactionError,
    FSETypedQueryIndexArchiveCompactionResult, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFileError, FSETypedQueryIndexArchiveLoadResult,
    append_typed_query_index_archive_file, build_typed_query_index_archive_file,
    build_typed_query_index_archive_file_with_encoder_metadata,
    compact_typed_query_index_archive_file, load_typed_query_index_archive_file,
    load_typed_query_index_archive_file_with_encoder_metadata,
    read_typed_query_index_archive_snapshot_file, save_typed_query_index_archive_file,
    save_typed_query_index_archive_file_with_encoder_metadata,
    write_typed_query_index_archive_snapshot_file,
};
pub use maintenance::{
    FSETypedQueryIndexArchiveMaintenanceError, FSETypedQueryIndexArchiveMaintenanceResult,
    maintain_typed_query_index_archive_file,
};
pub use snapshot::{FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError};
pub use tombstoned::{
    FSETombstonedTypedQueryIndex, FSETombstonedTypedQueryIndexArchiveError,
    load_typed_query_index_archive_with_tombstones,
};
