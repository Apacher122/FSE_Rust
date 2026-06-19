//! Dataset import workflows.
//!
//! Import workflows compose typed data parsing, semantic encoding, FSE index
//! construction, and archive persistence.

mod csv;

pub use csv::{
    FSECsvAppendDeltaArchiveImportError, FSECsvAppendDeltaArchiveMaintenanceImportError,
    FSECsvAppendDeltaArchiveQueryContext, FSECsvAppendDeltaArchiveQueryContextError,
    FSECsvAppendDeltaArchiveQueryError, FSECsvArchiveImportError,
    FSECsvArchiveMaintenanceImportError, FSECsvArchiveQueryContext, FSECsvArchiveQueryContextError,
    FSECsvArchiveQueryError, FSECsvInferredArchiveImportError, FSECsvInferredArchiveImportResult,
    FSECsvTombstoneImportError, FSECsvTombstoneMaintenanceImportError,
    FSECsvTombstoneMaintenanceImportResult, FSECsvTombstonedAppendDeltaArchiveQueryContext,
    FSECsvTombstonedAppendDeltaArchiveQueryContextError, FSECsvTombstonedArchiveQueryContext,
    FSECsvTombstonedArchiveQueryContextError, append_typed_query_index_archive_from_csv_file,
    append_typed_query_index_archive_from_csv_file_with_archive_metadata,
    append_typed_record_batch_archive_from_csv_file,
    append_typed_record_batch_archive_from_csv_file_with_archive_schema,
    append_typed_row_tombstone_archive_from_csv_file,
    build_typed_query_index_archive_from_csv_file,
    build_typed_query_index_archive_from_inferred_csv_file,
    build_typed_record_batch_archive_from_csv_file,
    build_typed_record_batch_archive_from_csv_file_with_archive_schema,
    load_csv_append_delta_typed_query_index_archive_context,
    load_csv_tombstoned_append_delta_typed_query_index_archive_context,
    load_csv_tombstoned_typed_query_index_archive_context,
    load_csv_typed_query_index_archive_context,
    maintain_typed_query_index_archive_from_append_delta_archive,
    maintain_typed_query_index_archive_from_csv_file,
    maintain_typed_query_index_archive_from_csv_file_with_archive_metadata,
    maintain_typed_query_index_archive_from_csv_tombstone_file,
};
