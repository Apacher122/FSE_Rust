//! CSV import workflows for typed query index archives.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::build::FSEBuilder;
use crate::data::{
    FSECsvFileImportError, FSECsvImportOptions, FSESchema, record_batch_from_csv_file,
};
use crate::encoding::FSERecordEncoder;
use crate::persistence::{
    FSETypedQueryIndexArchiveAppendResult, FSETypedQueryIndexArchiveError,
    append_typed_query_index_archive_file, build_typed_query_index_archive_file,
};
use crate::query::TypedQueryIndex;

/// Error returned when CSV archive import fails.
#[derive(Debug)]
pub enum FSECsvArchiveImportError {
    /// Reading or parsing the CSV file failed.
    Csv(FSECsvFileImportError),

    /// Building, appending, or writing the typed query index archive failed.
    Archive(FSETypedQueryIndexArchiveError),
}

impl fmt::Display for FSECsvArchiveImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => error.fmt(formatter),
            Self::Archive(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvArchiveImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::Archive(error) => Some(error),
        }
    }
}

impl From<FSECsvFileImportError> for FSECsvArchiveImportError {
    fn from(error: FSECsvFileImportError) -> Self {
        Self::Csv(error)
    }
}

impl From<FSETypedQueryIndexArchiveError> for FSECsvArchiveImportError {
    fn from(error: FSETypedQueryIndexArchiveError) -> Self {
        Self::Archive(error)
    }
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
