//! Typed archive metadata.

mod snapshot;

pub use snapshot::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
};
