//! Typed query index archive metadata.

mod codec;
mod compaction;
mod file;
mod footprint;
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
pub use footprint::{
    FSETypedQueryIndexArchiveFootprint, FSETypedQueryIndexArchiveFootprintComponent,
    FSETypedQueryIndexArchiveFootprintError, typed_query_index_archive_footprint,
    typed_query_index_archive_with_append_delta_and_tombstones_footprint,
    typed_query_index_archive_with_append_delta_footprint,
    typed_query_index_archive_with_tombstones_footprint,
};
pub use maintenance::{
    FSETypedQueryIndexAppendDeltaArchiveMaintenanceError,
    FSETypedQueryIndexArchiveMaintenanceError, FSETypedQueryIndexArchiveMaintenanceResult,
    inspect_typed_query_index_archive_file_maintenance,
    inspect_typed_query_index_archive_file_maintenance_with_append_batch_archive,
    maintain_typed_query_index_archive_file,
    maintain_typed_query_index_archive_file_with_append_batch_archive,
};
pub use snapshot::{FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError};
pub use tombstoned::{
    FSETombstonedTypedQueryIndex, FSETombstonedTypedQueryIndexArchiveError,
    load_typed_query_index_archive_with_tombstones,
};
