//! Typed record batch archive metadata.

mod codec;
mod file;
mod snapshot;

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
pub use snapshot::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
};
