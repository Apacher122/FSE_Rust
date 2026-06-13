//! Typed archive metadata.

mod codec;
mod file;
mod index_codec;
mod index_file;
mod index_snapshot;
mod snapshot;
mod tombstone;
mod tombstone_codec;
mod tombstone_file;

pub use codec::{
    FSETypedRecordBatchArchiveCodecError, decode_typed_record_batch_archive_snapshot,
    encode_typed_record_batch_archive_snapshot,
};
pub use file::{
    FSERecordBatchArchiveAppendResult, FSERecordBatchArchiveError, FSERecordBatchArchiveFileError,
    append_typed_record_batch_archive_file, load_typed_record_batch_archive_file,
    read_typed_record_batch_archive_snapshot_file, save_typed_record_batch_archive_file,
    write_typed_record_batch_archive_snapshot_file,
};
pub use index_codec::{
    FSETypedQueryIndexArchiveCodecError, decode_typed_query_index_archive_snapshot,
    encode_typed_query_index_archive_snapshot,
};
pub use index_file::{
    FSETypedQueryIndexArchiveAppendResult, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFileError, append_typed_query_index_archive_file,
    load_typed_query_index_archive_file, read_typed_query_index_archive_snapshot_file,
    save_typed_query_index_archive_file, write_typed_query_index_archive_snapshot_file,
};
pub use index_snapshot::{
    FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError,
};
pub use snapshot::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
};
pub use tombstone::{
    FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveSnapshotError,
};
pub use tombstone_codec::{
    FSETypedRowTombstoneArchiveCodecError, decode_typed_row_tombstone_archive_snapshot,
    encode_typed_row_tombstone_archive_snapshot,
};
pub use tombstone_file::{
    FSETypedRowTombstoneArchiveAppendResult, FSETypedRowTombstoneArchiveError,
    FSETypedRowTombstoneArchiveFileError, append_typed_row_tombstone_archive_file,
    load_typed_row_tombstone_archive_file, read_typed_row_tombstone_archive_snapshot_file,
    save_typed_row_tombstone_archive_file, write_typed_row_tombstone_archive_snapshot_file,
};
