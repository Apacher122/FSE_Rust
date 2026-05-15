//! Error handling for benchmark application execution.

use std::error::Error;
use std::fmt;

use crate::benchmark::reports::BenchmarkCsvWriteError;

/// Error returned by benchmark application orchestration.
///
/// # Runtime Role
///
/// `BenchmarkApplicationError` gives the binary entrypoint one error type for
/// failures that happen after CLI parsing succeeds.
#[derive(Debug)]
pub enum BenchmarkApplicationError {
    /// CSV output failed.
    CsvWrite(BenchmarkCsvWriteError),
}

impl fmt::Display for BenchmarkApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BenchmarkApplicationError::CsvWrite(error) => write!(formatter, "{}", error),
        }
    }
}

impl Error for BenchmarkApplicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BenchmarkApplicationError::CsvWrite(error) => Some(error),
        }
    }
}

impl From<BenchmarkCsvWriteError> for BenchmarkApplicationError {
    fn from(error: BenchmarkCsvWriteError) -> Self {
        Self::CsvWrite(error)
    }
}
