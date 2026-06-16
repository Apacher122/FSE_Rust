//! Dataset import workflows.
//!
//! Import workflows compose typed data parsing, semantic encoding, FSE index
//! construction, and archive persistence.

mod csv;

pub use csv::{
    FSECsvArchiveImportError, FSECsvArchiveMaintenanceImportError,
    FSECsvInferredArchiveImportError, FSECsvInferredArchiveImportResult,
    append_typed_query_index_archive_from_csv_file,
    append_typed_query_index_archive_from_csv_file_with_archive_metadata,
    build_typed_query_index_archive_from_csv_file,
    build_typed_query_index_archive_from_inferred_csv_file,
    maintain_typed_query_index_archive_from_csv_file,
    maintain_typed_query_index_archive_from_csv_file_with_archive_metadata,
};
