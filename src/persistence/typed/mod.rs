//! Typed archive metadata.

mod codec;
mod snapshot;

pub use codec::{
    FSETypedRecordBatchArchiveCodecError, decode_typed_record_batch_archive_snapshot,
    encode_typed_record_batch_archive_snapshot,
};
pub use snapshot::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
};
