//! Composed record encoders.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::data::{FSEFieldType, FSERecord, FSERecordBatch, FSESchema, FSEValue};

use super::{
    BooleanEncoder, CategoricalDictionaryEncoder, CategoricalDictionaryError, EncodedCoordinates,
    FSEEncodingError, FSEFieldEncoder, FSERecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};

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

/// Error returned when deriving a composed record encoder from a batch fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposedRecordEncoderFromBatchError {
    /// A field type has no default exact encoder.
    UnsupportedFieldType {
        /// Field index containing the unsupported type.
        field: usize,
        /// Field name containing the unsupported type.
        name: String,
        /// Unsupported field type.
        field_type: FSEFieldType,
    },

    /// A null value was found in a field without a null encoder.
    NullFieldValue {
        /// Record index containing the null value.
        record: usize,
        /// Field index containing the null value.
        field: usize,
        /// Field name containing the null value.
        name: String,
    },

    /// A categorical dictionary could not be built.
    CategoryDictionary {
        /// Field index containing the categorical value.
        field: usize,
        /// Field name containing the categorical value.
        name: String,
        /// Dictionary construction error.
        source: CategoricalDictionaryError,
    },

    /// The derived encoder did not satisfy the schema.
    Encoder(ComposedRecordEncoderError),
}

impl fmt::Display for ComposedRecordEncoderFromBatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFieldType {
                name, field_type, ..
            } => {
                write!(
                    formatter,
                    "field '{name}' with type {field_type:?} has no derived encoder"
                )
            }
            Self::NullFieldValue { record, name, .. } => {
                write!(
                    formatter,
                    "record {record} field '{name}' is null and has no derived encoder"
                )
            }
            Self::CategoryDictionary { source, .. } => source.fmt(formatter),
            Self::Encoder(error) => error.fmt(formatter),
        }
    }
}

impl Error for ComposedRecordEncoderFromBatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CategoryDictionary { source, .. } => Some(source),
            Self::Encoder(error) => Some(error),
            Self::UnsupportedFieldType { .. } | Self::NullFieldValue { .. } => None,
        }
    }
}

impl From<ComposedRecordEncoderError> for ComposedRecordEncoderFromBatchError {
    fn from(error: ComposedRecordEncoderError) -> Self {
        Self::Encoder(error)
    }
}

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

    /// Derives a composed record encoder from the typed values in a batch.
    ///
    /// Categorical dictionaries are built from observed category labels in
    /// record order.
    pub fn try_from_batch(
        batch: &FSERecordBatch,
    ) -> Result<Self, ComposedRecordEncoderFromBatchError> {
        let schema = batch.schema();
        let mut field_encoders = Vec::with_capacity(schema.len());

        validate_no_null_values(batch)?;

        for (field_index, field) in schema.fields().iter().enumerate() {
            let encoder: Box<dyn FSEFieldEncoder> = match field.field_type {
                FSEFieldType::Integer => Box::new(IntegerEncoder),
                FSEFieldType::Float => Box::new(FloatEncoder),
                FSEFieldType::Boolean => Box::new(BooleanEncoder),
                FSEFieldType::TimestampMillis => Box::new(TimestampMillisEncoder),
                FSEFieldType::Category => Box::new(category_encoder_from_batch(
                    batch,
                    field_index,
                    &field.name,
                )?),
                FSEFieldType::Text => {
                    return Err(ComposedRecordEncoderFromBatchError::UnsupportedFieldType {
                        field: field_index,
                        name: field.name.clone(),
                        field_type: field.field_type,
                    });
                }
            };

            field_encoders.push(encoder);
        }

        Ok(Self::try_new(schema, field_encoders)?)
    }

    /// Returns the number of field encoders.
    pub fn field_encoder_count(&self) -> usize {
        self.field_encoders.len()
    }
}

fn validate_no_null_values(
    batch: &FSERecordBatch,
) -> Result<(), ComposedRecordEncoderFromBatchError> {
    for (record_index, record) in batch.records().iter().enumerate() {
        for (field_index, (value, field)) in record
            .values()
            .iter()
            .zip(batch.schema().fields())
            .enumerate()
        {
            if matches!(value, FSEValue::Null) {
                return Err(ComposedRecordEncoderFromBatchError::NullFieldValue {
                    record: record_index,
                    field: field_index,
                    name: field.name.clone(),
                });
            }
        }
    }

    Ok(())
}

fn category_encoder_from_batch(
    batch: &FSERecordBatch,
    field_index: usize,
    field_name: &str,
) -> Result<CategoricalDictionaryEncoder, ComposedRecordEncoderFromBatchError> {
    let mut seen = HashSet::new();
    let mut categories = Vec::new();

    for record in batch.records() {
        let Some(FSEValue::Category(category)) = record.value(field_index) else {
            continue;
        };

        if seen.insert(category.as_str()) {
            categories.push(category.clone());
        }
    }

    CategoricalDictionaryEncoder::try_new(categories).map_err(|source| {
        ComposedRecordEncoderFromBatchError::CategoryDictionary {
            field: field_index,
            name: field_name.to_string(),
            source,
        }
    })
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
