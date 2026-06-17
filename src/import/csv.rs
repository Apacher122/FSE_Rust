//! CSV import workflows for typed query index archives.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::build::FSEBuilder;
use crate::data::{
    FSECsvFileImportError, FSECsvImportOptions, FSECsvSchemaInferenceOptions, FSESchema,
    infer_schema_from_csv_file_with_options, record_batch_from_csv_file, row_ids_from_csv_file,
};
use crate::encoding::{
    ComposedRecordEncoderFromBatchError, FSERecordEncoder, FSERecordEncoderMetadata,
    FSERecordEncoderMetadataError,
};
use crate::persistence::{
    FSEArchiveMaintenancePolicy, FSETypedQueryIndexArchiveAppendResult,
    FSETypedQueryIndexArchiveError, FSETypedQueryIndexArchiveMaintenanceError,
    FSETypedQueryIndexArchiveMaintenanceResult, FSETypedRowTombstoneArchiveAppendResult,
    FSETypedRowTombstoneArchiveError, append_typed_query_index_archive_file,
    append_typed_row_tombstone_archive_file, build_typed_query_index_archive_file,
    build_typed_query_index_archive_file_with_encoder_metadata,
    load_typed_query_index_archive_file_with_encoder_metadata,
    maintain_typed_query_index_archive_file,
};
use crate::query::TypedQueryIndex;

/// Error returned when CSV archive import fails.
#[derive(Debug)]
pub enum FSECsvArchiveImportError {
    /// Reading or parsing the CSV file failed.
    Csv(FSECsvFileImportError),

    /// Stored record encoder metadata could not rebuild a runtime encoder.
    EncoderMetadata(FSERecordEncoderMetadataError),

    /// Building, appending, or writing the typed query index archive failed.
    Archive(FSETypedQueryIndexArchiveError),
}

impl fmt::Display for FSECsvArchiveImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => error.fmt(formatter),
            Self::EncoderMetadata(error) => error.fmt(formatter),
            Self::Archive(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvArchiveImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::EncoderMetadata(error) => Some(error),
            Self::Archive(error) => Some(error),
        }
    }
}

impl From<FSECsvFileImportError> for FSECsvArchiveImportError {
    fn from(error: FSECsvFileImportError) -> Self {
        Self::Csv(error)
    }
}

impl From<FSERecordEncoderMetadataError> for FSECsvArchiveImportError {
    fn from(error: FSERecordEncoderMetadataError) -> Self {
        Self::EncoderMetadata(error)
    }
}

impl From<FSETypedQueryIndexArchiveError> for FSECsvArchiveImportError {
    fn from(error: FSETypedQueryIndexArchiveError) -> Self {
        Self::Archive(error)
    }
}

/// Error returned when CSV tombstone import fails.
#[derive(Debug)]
pub enum FSECsvTombstoneImportError {
    /// Reading or parsing the CSV file failed.
    Csv(FSECsvFileImportError),

    /// Appending row tombstones to the archive failed.
    Tombstones(FSETypedRowTombstoneArchiveError),
}

impl fmt::Display for FSECsvTombstoneImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => error.fmt(formatter),
            Self::Tombstones(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvTombstoneImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::Tombstones(error) => Some(error),
        }
    }
}

impl From<FSECsvFileImportError> for FSECsvTombstoneImportError {
    fn from(error: FSECsvFileImportError) -> Self {
        Self::Csv(error)
    }
}

impl From<FSETypedRowTombstoneArchiveError> for FSECsvTombstoneImportError {
    fn from(error: FSETypedRowTombstoneArchiveError) -> Self {
        Self::Tombstones(error)
    }
}

/// Error returned when inferred CSV archive import fails.
#[derive(Debug)]
pub enum FSECsvInferredArchiveImportError {
    /// Reading, schema inference, or parsing the CSV file failed.
    Csv(FSECsvFileImportError),

    /// Deriving a record encoder from the parsed batch failed.
    Encoder(ComposedRecordEncoderFromBatchError),

    /// Building a record encoder from derived metadata failed.
    EncoderMetadata(FSERecordEncoderMetadataError),

    /// Building or writing the typed query index archive failed.
    Archive(FSETypedQueryIndexArchiveError),
}

impl fmt::Display for FSECsvInferredArchiveImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => error.fmt(formatter),
            Self::Encoder(error) => error.fmt(formatter),
            Self::EncoderMetadata(error) => error.fmt(formatter),
            Self::Archive(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvInferredArchiveImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::Encoder(error) => Some(error),
            Self::EncoderMetadata(error) => Some(error),
            Self::Archive(error) => Some(error),
        }
    }
}

impl From<FSECsvFileImportError> for FSECsvInferredArchiveImportError {
    fn from(error: FSECsvFileImportError) -> Self {
        Self::Csv(error)
    }
}

impl From<ComposedRecordEncoderFromBatchError> for FSECsvInferredArchiveImportError {
    fn from(error: ComposedRecordEncoderFromBatchError) -> Self {
        Self::Encoder(error)
    }
}

impl From<FSERecordEncoderMetadataError> for FSECsvInferredArchiveImportError {
    fn from(error: FSERecordEncoderMetadataError) -> Self {
        Self::EncoderMetadata(error)
    }
}

impl From<FSETypedQueryIndexArchiveError> for FSECsvInferredArchiveImportError {
    fn from(error: FSETypedQueryIndexArchiveError) -> Self {
        Self::Archive(error)
    }
}

/// Result returned after building a typed query index archive from inferred CSV metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct FSECsvInferredArchiveImportResult {
    /// Schema inferred from the CSV header and values.
    pub schema: FSESchema,

    /// Record encoder metadata derived from the parsed record batch.
    pub record_encoder_metadata: FSERecordEncoderMetadata,

    /// Query index written to the archive.
    pub query_index: TypedQueryIndex,
}

/// Error returned when CSV archive maintenance import fails.
#[derive(Debug)]
pub enum FSECsvArchiveMaintenanceImportError {
    /// Reading or parsing the CSV file failed.
    Csv(FSECsvFileImportError),

    /// Stored record encoder metadata could not rebuild a runtime encoder.
    EncoderMetadata(FSERecordEncoderMetadataError),

    /// Typed query index archive maintenance failed.
    Maintenance(FSETypedQueryIndexArchiveMaintenanceError),
}

impl fmt::Display for FSECsvArchiveMaintenanceImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => error.fmt(formatter),
            Self::EncoderMetadata(error) => error.fmt(formatter),
            Self::Maintenance(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvArchiveMaintenanceImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::EncoderMetadata(error) => Some(error),
            Self::Maintenance(error) => Some(error),
        }
    }
}

impl From<FSECsvFileImportError> for FSECsvArchiveMaintenanceImportError {
    fn from(error: FSECsvFileImportError) -> Self {
        Self::Csv(error)
    }
}

impl From<FSERecordEncoderMetadataError> for FSECsvArchiveMaintenanceImportError {
    fn from(error: FSERecordEncoderMetadataError) -> Self {
        Self::EncoderMetadata(error)
    }
}

impl From<FSETypedQueryIndexArchiveMaintenanceError> for FSECsvArchiveMaintenanceImportError {
    fn from(error: FSETypedQueryIndexArchiveMaintenanceError) -> Self {
        Self::Maintenance(error)
    }
}

/// Error returned when CSV tombstone maintenance import fails.
#[derive(Debug)]
pub enum FSECsvTombstoneMaintenanceImportError {
    /// Reading or parsing the CSV file failed.
    Csv(FSECsvFileImportError),

    /// Stored record encoder metadata could not rebuild a runtime encoder.
    EncoderMetadata(FSERecordEncoderMetadataError),

    /// Appending row tombstones to the archive failed.
    Tombstones(FSETypedRowTombstoneArchiveError),

    /// Typed query index archive maintenance failed.
    Maintenance(FSETypedQueryIndexArchiveMaintenanceError),
}

impl fmt::Display for FSECsvTombstoneMaintenanceImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => error.fmt(formatter),
            Self::EncoderMetadata(error) => error.fmt(formatter),
            Self::Tombstones(error) => error.fmt(formatter),
            Self::Maintenance(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvTombstoneMaintenanceImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::EncoderMetadata(error) => Some(error),
            Self::Tombstones(error) => Some(error),
            Self::Maintenance(error) => Some(error),
        }
    }
}

impl From<FSECsvFileImportError> for FSECsvTombstoneMaintenanceImportError {
    fn from(error: FSECsvFileImportError) -> Self {
        Self::Csv(error)
    }
}

impl From<FSERecordEncoderMetadataError> for FSECsvTombstoneMaintenanceImportError {
    fn from(error: FSERecordEncoderMetadataError) -> Self {
        Self::EncoderMetadata(error)
    }
}

impl From<FSETypedRowTombstoneArchiveError> for FSECsvTombstoneMaintenanceImportError {
    fn from(error: FSETypedRowTombstoneArchiveError) -> Self {
        Self::Tombstones(error)
    }
}

impl From<FSETypedQueryIndexArchiveMaintenanceError> for FSECsvTombstoneMaintenanceImportError {
    fn from(error: FSETypedQueryIndexArchiveMaintenanceError) -> Self {
        Self::Maintenance(error)
    }
}

/// Result returned after importing CSV row tombstones and applying maintenance.
#[derive(Clone, Debug, PartialEq)]
pub struct FSECsvTombstoneMaintenanceImportResult {
    /// Result returned after appending row tombstones from the CSV file.
    pub tombstone_append: FSETypedRowTombstoneArchiveAppendResult,

    /// Result returned after applying archive maintenance.
    pub maintenance: FSETypedQueryIndexArchiveMaintenanceResult,
}

/// Builds a typed query index archive from a CSV file.
///
/// The caller supplies the schema, CSV row-id options, record encoder, and FSE
/// builder used for archive construction.
pub fn build_typed_query_index_archive_from_csv_file<C, A>(
    csv_path: C,
    archive_path: A,
    schema: &FSESchema,
    options: &FSECsvImportOptions,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<TypedQueryIndex, FSECsvArchiveImportError>
where
    C: AsRef<Path>,
    A: AsRef<Path>,
{
    let batch = record_batch_from_csv_file(csv_path, schema, options)?;

    Ok(build_typed_query_index_archive_file(
        archive_path,
        batch,
        encoder,
        builder,
    )?)
}

/// Builds a typed query index archive from a CSV file with inferred schema metadata.
///
/// The schema is inferred from the CSV file, the record encoder is derived from
/// the parsed record batch, and the resulting typed query index is written to
/// the archive path.
pub fn build_typed_query_index_archive_from_inferred_csv_file<C, A>(
    csv_path: C,
    archive_path: A,
    schema_options: &FSECsvSchemaInferenceOptions,
    import_options: &FSECsvImportOptions,
    builder: &FSEBuilder,
) -> Result<FSECsvInferredArchiveImportResult, FSECsvInferredArchiveImportError>
where
    C: AsRef<Path>,
    A: AsRef<Path>,
{
    let csv_path = csv_path.as_ref();
    let archive_path = archive_path.as_ref();
    let schema = infer_schema_from_csv_file_with_options(csv_path, schema_options)?;
    let batch = record_batch_from_csv_file(csv_path, &schema, import_options)?;
    let record_encoder_metadata = FSERecordEncoderMetadata::from_batch(&batch)?;
    let encoder = record_encoder_metadata.to_record_encoder(&schema)?;
    let query_index = build_typed_query_index_archive_file_with_encoder_metadata(
        archive_path,
        batch,
        &encoder,
        record_encoder_metadata.clone(),
        builder,
    )?;

    Ok(FSECsvInferredArchiveImportResult {
        schema,
        record_encoder_metadata,
        query_index,
    })
}

/// Appends CSV records to an existing typed query index archive.
///
/// The CSV file is parsed with the caller supplied schema and row-id options.
/// The parsed record batch is appended to the archive through the typed query
/// index archive append workflow.
pub fn append_typed_query_index_archive_from_csv_file<C, A>(
    csv_path: C,
    archive_path: A,
    schema: &FSESchema,
    options: &FSECsvImportOptions,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<FSETypedQueryIndexArchiveAppendResult, FSECsvArchiveImportError>
where
    C: AsRef<Path>,
    A: AsRef<Path>,
{
    let batch = record_batch_from_csv_file(csv_path, schema, options)?;

    Ok(append_typed_query_index_archive_file(
        archive_path,
        &batch,
        encoder,
        builder,
    )?)
}

/// Appends CSV records using schema and encoder metadata stored in the archive.
///
/// The archive supplies the schema used for CSV parsing and the record encoder
/// metadata used for index rebuilds.
pub fn append_typed_query_index_archive_from_csv_file_with_archive_metadata<C, A>(
    csv_path: C,
    archive_path: A,
    options: &FSECsvImportOptions,
    builder: &FSEBuilder,
) -> Result<FSETypedQueryIndexArchiveAppendResult, FSECsvArchiveImportError>
where
    C: AsRef<Path>,
    A: AsRef<Path>,
{
    let archive_path = archive_path.as_ref();
    let loaded = load_typed_query_index_archive_file_with_encoder_metadata(archive_path)?;
    let schema = loaded.query_index.batch().schema();
    let batch = record_batch_from_csv_file(csv_path, schema, options)?;
    let encoder = loaded.record_encoder_metadata.to_record_encoder(schema)?;

    Ok(append_typed_query_index_archive_file(
        archive_path,
        &batch,
        &encoder,
        builder,
    )?)
}

/// Appends row tombstones from a CSV file to a typed row tombstone archive.
///
/// The CSV file is parsed with the caller supplied row-id options. Parsed row
/// identifiers are appended to the tombstone archive.
pub fn append_typed_row_tombstone_archive_from_csv_file<C, T>(
    csv_path: C,
    tombstone_path: T,
    options: &FSECsvImportOptions,
) -> Result<FSETypedRowTombstoneArchiveAppendResult, FSECsvTombstoneImportError>
where
    C: AsRef<Path>,
    T: AsRef<Path>,
{
    let row_ids = row_ids_from_csv_file(csv_path, options)?;

    Ok(append_typed_row_tombstone_archive_file(
        tombstone_path,
        &row_ids,
    )?)
}

/// Imports row tombstones from CSV and applies typed query index archive maintenance.
///
/// The CSV file supplies row identifiers that are appended to the tombstone
/// archive. The persisted query index archive supplies schema and encoder
/// metadata used by maintenance.
pub fn maintain_typed_query_index_archive_from_csv_tombstone_file<C, A, T>(
    csv_path: C,
    archive_path: A,
    tombstone_path: T,
    options: &FSECsvImportOptions,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
) -> Result<FSECsvTombstoneMaintenanceImportResult, FSECsvTombstoneMaintenanceImportError>
where
    C: AsRef<Path>,
    A: AsRef<Path>,
    T: AsRef<Path>,
{
    let archive_path = archive_path.as_ref();
    let tombstone_path = tombstone_path.as_ref();
    let row_ids = row_ids_from_csv_file(csv_path, options)?;
    let loaded = load_typed_query_index_archive_file_with_encoder_metadata(archive_path)
        .map_err(FSETypedQueryIndexArchiveMaintenanceError::LoadIndex)?;
    let schema = loaded.query_index.batch().schema();
    let encoder = loaded.record_encoder_metadata.to_record_encoder(schema)?;
    let tombstone_append = append_typed_row_tombstone_archive_file(tombstone_path, &row_ids)?;
    let maintenance = maintain_typed_query_index_archive_file(
        archive_path,
        tombstone_path,
        None,
        &encoder,
        builder,
        policy,
    )?;

    Ok(FSECsvTombstoneMaintenanceImportResult {
        tombstone_append,
        maintenance,
    })
}

/// Applies typed query index archive maintenance with CSV append records.
///
/// The CSV file supplies pending append records for the maintenance policy.
/// Tombstones are loaded from the caller supplied tombstone archive path.
pub fn maintain_typed_query_index_archive_from_csv_file<C, A, T>(
    csv_path: C,
    archive_path: A,
    tombstone_path: T,
    schema: &FSESchema,
    options: &FSECsvImportOptions,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
) -> Result<FSETypedQueryIndexArchiveMaintenanceResult, FSECsvArchiveMaintenanceImportError>
where
    C: AsRef<Path>,
    A: AsRef<Path>,
    T: AsRef<Path>,
{
    let batch = record_batch_from_csv_file(csv_path, schema, options)?;

    Ok(maintain_typed_query_index_archive_file(
        archive_path,
        tombstone_path,
        Some(&batch),
        encoder,
        builder,
        policy,
    )?)
}

/// Applies archive maintenance using schema and encoder metadata stored in the archive.
///
/// The archive supplies the schema used for CSV parsing and the record encoder
/// metadata used for maintenance rebuilds.
pub fn maintain_typed_query_index_archive_from_csv_file_with_archive_metadata<C, A, T>(
    csv_path: C,
    archive_path: A,
    tombstone_path: T,
    options: &FSECsvImportOptions,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
) -> Result<FSETypedQueryIndexArchiveMaintenanceResult, FSECsvArchiveMaintenanceImportError>
where
    C: AsRef<Path>,
    A: AsRef<Path>,
    T: AsRef<Path>,
{
    let archive_path = archive_path.as_ref();
    let loaded = load_typed_query_index_archive_file_with_encoder_metadata(archive_path)
        .map_err(FSETypedQueryIndexArchiveMaintenanceError::LoadIndex)?;
    let schema = loaded.query_index.batch().schema();
    let batch = record_batch_from_csv_file(csv_path, schema, options)?;
    let encoder = loaded.record_encoder_metadata.to_record_encoder(schema)?;

    Ok(maintain_typed_query_index_archive_file(
        archive_path,
        tombstone_path,
        Some(&batch),
        &encoder,
        builder,
        policy,
    )?)
}
