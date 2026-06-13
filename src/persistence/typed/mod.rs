//! Typed archive metadata.

mod query_index;
mod record_batch;
mod tombstone;

pub use query_index::{
    FSETombstonedTypedQueryIndex, FSETombstonedTypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveAppendResult, FSETypedQueryIndexArchiveCodecError,
    FSETypedQueryIndexArchiveError, FSETypedQueryIndexArchiveFileError,
    FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError,
    append_typed_query_index_archive_file, decode_typed_query_index_archive_snapshot,
    encode_typed_query_index_archive_snapshot, load_typed_query_index_archive_file,
    load_typed_query_index_archive_with_tombstones, read_typed_query_index_archive_snapshot_file,
    save_typed_query_index_archive_file, write_typed_query_index_archive_snapshot_file,
};
pub use record_batch::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveAppendResult, FSERecordBatchArchiveError, FSERecordBatchArchiveFileError,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveCodecError,
    FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
    append_typed_record_batch_archive_file, decode_typed_record_batch_archive_snapshot,
    encode_typed_record_batch_archive_snapshot, load_typed_record_batch_archive_file,
    read_typed_record_batch_archive_snapshot_file, save_typed_record_batch_archive_file,
    write_typed_record_batch_archive_snapshot_file,
};
pub use tombstone::{
    FSETypedRowTombstoneArchiveAppendResult, FSETypedRowTombstoneArchiveCodecError,
    FSETypedRowTombstoneArchiveError, FSETypedRowTombstoneArchiveFileError,
    FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveSnapshotError,
    append_typed_row_tombstone_archive_file, decode_typed_row_tombstone_archive_snapshot,
    encode_typed_row_tombstone_archive_snapshot, load_typed_row_tombstone_archive_file,
    read_typed_row_tombstone_archive_snapshot_file, save_typed_row_tombstone_archive_file,
    write_typed_row_tombstone_archive_snapshot_file,
};
