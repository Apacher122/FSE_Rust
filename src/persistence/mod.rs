//! Persistence components for FSE archives.
//!
//! This module defines versioned archive metadata, numeric index archives,
//! row-mapped archive metadata, and typed record archive metadata used by
//! durable FSE storage.

mod append;
mod archive;
mod manifest;
mod payload;
mod row_mapped;
mod typed;

pub use append::{
    FSEArchiveAppendOperationMetadata, FSEArchiveAppendOperationMetadataError,
    FSEArchiveRebuildPlanMetadata, FSEArchiveRebuildPlanMetadataError, FSEArchiveRebuildReason,
};
pub use archive::{
    FSEArchiveCodecError, FSEArchiveFileError, FSEArchiveFileOperation, FSEArchiveSnapshotError,
    FSEIndexArchiveError, FSEIndexArchiveSnapshot, FSEPartitionNodeArchiveRecord,
    decode_archive_snapshot, encode_archive_snapshot, load_index_archive_file,
    read_archive_snapshot_file, save_index_archive_file, save_index_archive_file_with_sections,
    write_archive_snapshot_file,
};
pub use manifest::{
    FSE_ARCHIVE_FILE_EXTENSION, FSE_ARCHIVE_FORMAT_VERSION, FSE_ARCHIVE_MAGIC, FSEArchiveManifest,
    FSEArchiveManifestError, FSEArchiveSections,
};
pub use payload::{
    FSE_ARCHIVE_PAYLOAD_HEADER_VERSION, FSE_ARCHIVE_PAYLOAD_MAGIC, FSEArchivePayloadHeaderError,
    FSEArchivePayloadKind, FSEArchivePayloadMetadata, decode_archive_payload,
    encode_archive_payload, inspect_archive_payload,
};
pub use row_mapped::{
    FSELeafRowIdArchiveRecord, FSERowMappedArchiveCodecError, FSERowMappedArchiveFileError,
    FSERowMappedArchiveSnapshotError, FSERowMappedIndexArchiveError,
    FSERowMappedIndexArchiveSnapshot, decode_row_mapped_archive_snapshot,
    encode_row_mapped_archive_snapshot, load_row_mapped_index_archive_file,
    read_row_mapped_archive_snapshot_file, save_row_mapped_index_archive_file,
    write_row_mapped_archive_snapshot_file,
};
pub use typed::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveError, FSERecordBatchArchiveFileError, FSERecordBatchArchiveSnapshot,
    FSETypedQueryIndexArchiveCodecError, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFileError, FSETypedQueryIndexArchiveSnapshot,
    FSETypedQueryIndexArchiveSnapshotError, FSETypedRecordBatchArchiveCodecError,
    FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
    decode_typed_query_index_archive_snapshot, decode_typed_record_batch_archive_snapshot,
    encode_typed_query_index_archive_snapshot, encode_typed_record_batch_archive_snapshot,
    load_typed_query_index_archive_file, load_typed_record_batch_archive_file,
    read_typed_query_index_archive_snapshot_file, read_typed_record_batch_archive_snapshot_file,
    save_typed_query_index_archive_file, save_typed_record_batch_archive_file,
    write_typed_query_index_archive_snapshot_file, write_typed_record_batch_archive_snapshot_file,
};
