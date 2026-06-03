//! Encoding error types.

use std::error::Error;
use std::fmt;

use crate::data::FSEFieldType;

/// Error returned when typed values cannot be encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEEncodingError {
    /// The encoder received a null value that it cannot encode.
    NullValue,

    /// The value type did not match the encoder input type.
    FieldTypeMismatch {
        /// Field type expected by the encoder.
        expected: FSEFieldType,
        /// Field type found in the value.
        actual: FSEFieldType,
    },

    /// The value cannot be represented by the encoder.
    UnsupportedValue {
        /// Explanation suitable for diagnostics.
        reason: String,
    },
}

impl fmt::Display for FSEEncodingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullValue => formatter.write_str("encoder cannot encode null value"),
            Self::FieldTypeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "encoder expected {expected:?} but found {actual:?}"
                )
            }
            Self::UnsupportedValue { reason } => write!(formatter, "{reason}"),
        }
    }
}

impl Error for FSEEncodingError {}
