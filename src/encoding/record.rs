//! Composed record encoders.

use std::error::Error;
use std::fmt;

use crate::data::{FSEFieldType, FSERecord, FSESchema};

use super::{EncodedCoordinates, FSEEncodingError, FSEFieldEncoder, FSERecordEncoder};

/// Error returned when checked composed record encoder construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposedRecordEncoderError {
    /// The number of field encoders did not match the schema field count.
    EncoderCountMismatch {
        /// Number of field encoders provided.
        encoder_count: usize,
        /// Number of fields required by the schema.
        field_count: usize,
    },

    /// A field encoder accepted a different type than the corresponding schema field.
    FieldTypeMismatch {
        /// Field index containing the mismatched encoder.
        field: usize,
        /// Field name containing the mismatched encoder.
        name: String,
        /// Field type required by the schema.
        expected: FSEFieldType,
        /// Field type accepted by the encoder.
        actual: FSEFieldType,
    },
}

impl fmt::Display for ComposedRecordEncoderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EncoderCountMismatch {
                encoder_count,
                field_count,
            } => write!(
                formatter,
                "record encoder has {encoder_count} field encoders but schema requires {field_count}"
            ),
            Self::FieldTypeMismatch {
                name,
                expected,
                actual,
                ..
            } => write!(
                formatter,
                "field '{name}' encoder expected {expected:?} but found {actual:?}"
            ),
        }
    }
}

impl Error for ComposedRecordEncoderError {}

/// Record encoder composed from schema-ordered field encoders.
///
/// # Runtime Role
///
/// `ComposedRecordEncoder` maps one validated typed record into numeric
/// coordinates by encoding each field in schema order and concatenating the
/// resulting coordinate values.
pub struct ComposedRecordEncoder {
    field_encoders: Vec<Box<dyn FSEFieldEncoder>>,
    output_dimensions: usize,
}

impl ComposedRecordEncoder {
    /// Creates a composed record encoder.
    ///
    /// # Panics
    ///
    /// Panics when encoder count or field types do not match the schema.
    pub fn new(schema: &FSESchema, field_encoders: Vec<Box<dyn FSEFieldEncoder>>) -> Self {
        Self::try_new(schema, field_encoders).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a composed record encoder and returns an error when invalid.
    pub fn try_new(
        schema: &FSESchema,
        field_encoders: Vec<Box<dyn FSEFieldEncoder>>,
    ) -> Result<Self, ComposedRecordEncoderError> {
        if field_encoders.len() != schema.len() {
            return Err(ComposedRecordEncoderError::EncoderCountMismatch {
                encoder_count: field_encoders.len(),
                field_count: schema.len(),
            });
        }

        for (field_index, (field, encoder)) in schema
            .fields()
            .iter()
            .zip(field_encoders.iter())
            .enumerate()
        {
            let actual = encoder.field_type();

            if actual != field.field_type {
                return Err(ComposedRecordEncoderError::FieldTypeMismatch {
                    field: field_index,
                    name: field.name.clone(),
                    expected: field.field_type,
                    actual,
                });
            }
        }

        let output_dimensions = field_encoders
            .iter()
            .map(|encoder| encoder.output_dimensions())
            .sum();

        Ok(Self {
            field_encoders,
            output_dimensions,
        })
    }

    /// Returns the number of field encoders.
    pub fn field_encoder_count(&self) -> usize {
        self.field_encoders.len()
    }
}

impl FSERecordEncoder for ComposedRecordEncoder {
    fn output_dimensions(&self) -> usize {
        self.output_dimensions
    }

    fn encode_record(&self, record: &FSERecord) -> Result<EncodedCoordinates, FSEEncodingError> {
        let mut values = Vec::with_capacity(self.output_dimensions);

        for (field_index, encoder) in self.field_encoders.iter().enumerate() {
            let Some(value) = record.value(field_index) else {
                return Err(FSEEncodingError::UnsupportedValue {
                    reason: format!("record is missing field {field_index}"),
                });
            };

            let encoded = encoder.encode_value(value)?;
            values.extend_from_slice(encoded.values());
        }

        Ok(EncodedCoordinates::new(values))
    }
}
