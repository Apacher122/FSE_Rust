//! Typed archive metadata.

mod codec;
mod file;
mod index_snapshot;
mod snapshot;

pub use codec::{
    FSETypedRecordBatchArchiveCodecError, decode_typed_record_batch_archive_snapshot,
    encode_typed_record_batch_archive_snapshot,
};
pub use file::{
    FSERecordBatchArchiveError, FSERecordBatchArchiveFileError,
    load_typed_record_batch_archive_file, read_typed_record_batch_archive_snapshot_file,
    save_typed_record_batch_archive_file, write_typed_record_batch_archive_snapshot_file,
};
pub use index_snapshot::{
    FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError,
};
pub use snapshot::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
};
