//! Encoder traits.

use crate::data::{FSEFieldType, FSERecord, FSEValue};

use super::{EncodedCoordinates, FSEEncodingError};

/// Encoder for one logical value.
///
/// # Runtime Role
///
/// Field encoders define how one typed value contributes numeric coordinates
/// while preserving the semantics required for exact evaluation.
pub trait FSEFieldEncoder {
    /// Returns the input field type accepted by this encoder.
    fn field_type(&self) -> FSEFieldType;

    /// Returns the number of coordinate dimensions produced by this encoder.
    fn output_dimensions(&self) -> usize;

    /// Encodes one typed value into numeric coordinates.
    fn encode_value(&self, value: &FSEValue) -> Result<EncodedCoordinates, FSEEncodingError>;
}

/// Encoder for a complete typed record.
///
/// # Runtime Role
///
/// Record encoders map validated logical records into the coordinate space used
/// by the FSE geometry layer.
pub trait FSERecordEncoder {
    /// Returns the number of coordinate dimensions produced for each record.
    fn output_dimensions(&self) -> usize;

    /// Encodes one typed record into numeric coordinates.
    fn encode_record(&self, record: &FSERecord) -> Result<EncodedCoordinates, FSEEncodingError>;
}
