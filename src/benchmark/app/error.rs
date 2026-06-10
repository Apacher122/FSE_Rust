//! Error handling for benchmark application execution.

use std::error::Error;
use std::fmt;
use std::io;

use crate::benchmark::reports::{BenchmarkCsvWriteError, TypedArchiveLoadTimingError};
use crate::build::BuildValidationError;
use crate::persistence::FSEArchivePayloadHeaderError;

/// Error returned by benchmark application orchestration.
///
/// # Runtime Role
///
/// `BenchmarkApplicationError` gives the binary entrypoint one error type for
/// failures that happen after CLI parsing succeeds.
#[derive(Debug)]
pub enum BenchmarkApplicationError {
    /// Checked index construction failed validation.
    BuildValidation(BuildValidationError),

    /// CSV output failed.
    CsvWrite(BenchmarkCsvWriteError),

    /// Typed query index archive validation failed.
    TypedQueryIndexArchiveValidation(TypedArchiveLoadTimingError),

    /// Reading typed query index archive metadata failed.
    TypedQueryIndexArchiveMetadataRead {
        /// Path used by the read operation.
        path: String,

        /// Operating-system error kind.
        kind: io::ErrorKind,
    },

    /// Typed query index archive payload metadata is invalid.
    TypedQueryIndexArchivePayload(FSEArchivePayloadHeaderError),
}

impl fmt::Display for BenchmarkApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BenchmarkApplicationError::BuildValidation(error) => write!(formatter, "{}", error),
            BenchmarkApplicationError::CsvWrite(error) => write!(formatter, "{}", error),
            BenchmarkApplicationError::TypedQueryIndexArchiveValidation(error) => {
                write!(formatter, "{}", error)
            }
            BenchmarkApplicationError::TypedQueryIndexArchiveMetadataRead { .. } => {
                formatter.write_str("failed to read typed query index archive metadata")
            }
            BenchmarkApplicationError::TypedQueryIndexArchivePayload(error) => {
                write!(formatter, "{}", error)
            }
        }
    }
}

impl Error for BenchmarkApplicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BenchmarkApplicationError::BuildValidation(error) => Some(error),
            BenchmarkApplicationError::CsvWrite(error) => Some(error),
            BenchmarkApplicationError::TypedQueryIndexArchiveValidation(error) => Some(error),
            BenchmarkApplicationError::TypedQueryIndexArchivePayload(error) => Some(error),
            BenchmarkApplicationError::TypedQueryIndexArchiveMetadataRead { .. } => None,
        }
    }
}

impl From<BuildValidationError> for BenchmarkApplicationError {
    fn from(error: BuildValidationError) -> Self {
        Self::BuildValidation(error)
    }
}

impl From<BenchmarkCsvWriteError> for BenchmarkApplicationError {
    fn from(error: BenchmarkCsvWriteError) -> Self {
        Self::CsvWrite(error)
    }
}

impl From<TypedArchiveLoadTimingError> for BenchmarkApplicationError {
    fn from(error: TypedArchiveLoadTimingError) -> Self {
        Self::TypedQueryIndexArchiveValidation(error)
    }
}

impl From<FSEArchivePayloadHeaderError> for BenchmarkApplicationError {
    fn from(error: FSEArchivePayloadHeaderError) -> Self {
        Self::TypedQueryIndexArchivePayload(error)
    }
}
