//! Encoder metadata for FSE-native typed records.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::data::{FSEFieldType, FSERecordBatch, FSESchema, FSEValue};

use super::record::{
    ComposedRecordEncoder, ComposedRecordEncoderError, ComposedRecordEncoderFromBatchError,
};
use super::{
    BooleanEncoder, CategoricalDictionaryEncoder, CategoricalDictionaryError, FSEFieldEncoder,
    FloatEncoder, IntegerEncoder, TimestampMillisEncoder,
};

/// Error returned when record encoder metadata cannot build a runtime encoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERecordEncoderMetadataError {
    /// The number of field metadata records did not match the schema field count.
    FieldCountMismatch {
        /// Number of field metadata records.
        metadata_count: usize,
        /// Number of fields required by the schema.
        field_count: usize,
    },

    /// A field metadata record had a different type than the schema field.
    FieldTypeMismatch {
        /// Field index containing the mismatch.
        field: usize,
        /// Field name containing the mismatch.
        name: String,
        /// Field type required by the schema.
        expected: FSEFieldType,
        /// Field type stored by the metadata.
        actual: FSEFieldType,
    },

    /// A categorical dictionary metadata record was invalid.
    CategoryDictionary {
        /// Field index containing the categorical dictionary.
        field: usize,
        /// Field name containing the categorical dictionary.
        name: String,
        /// Dictionary validation error.
        source: CategoricalDictionaryError,
    },

    /// The constructed record encoder did not satisfy the schema.
    Encoder(ComposedRecordEncoderError),
}

impl fmt::Display for FSERecordEncoderMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldCountMismatch {
                metadata_count,
                field_count,
            } => write!(
                formatter,
                "record encoder metadata has {metadata_count} fields but schema requires {field_count}"
            ),
            Self::FieldTypeMismatch {
                name,
                expected,
                actual,
                ..
            } => write!(
                formatter,
                "field '{name}' metadata expected {expected:?} but found {actual:?}"
            ),
            Self::CategoryDictionary { source, .. } => source.fmt(formatter),
            Self::Encoder(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSERecordEncoderMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CategoryDictionary { source, .. } => Some(source),
            Self::Encoder(error) => Some(error),
            Self::FieldCountMismatch { .. } | Self::FieldTypeMismatch { .. } => None,
        }
    }
}

impl From<ComposedRecordEncoderError> for FSERecordEncoderMetadataError {
    fn from(error: ComposedRecordEncoderError) -> Self {
        Self::Encoder(error)
    }
}

/// Schema-ordered metadata for one field encoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEFieldEncoderMetadata {
    /// Signed integer encoder.
    Integer,

    /// Floating point encoder.
    Float,

    /// Boolean encoder.
    Boolean,

    /// Timestamp encoder for millisecond values.
    TimestampMillis,

    /// Dictionary-backed categorical encoder.
    CategoryDictionary {
        /// Category labels in dictionary order.
        categories: Vec<String>,
    },
}

impl FSEFieldEncoderMetadata {
    /// Returns the field type represented by this encoder metadata.
    pub fn field_type(&self) -> FSEFieldType {
        match self {
            Self::Integer => FSEFieldType::Integer,
            Self::Float => FSEFieldType::Float,
            Self::Boolean => FSEFieldType::Boolean,
            Self::TimestampMillis => FSEFieldType::TimestampMillis,
            Self::CategoryDictionary { .. } => FSEFieldType::Category,
        }
    }

    fn to_field_encoder(
        &self,
        field_index: usize,
        field_name: &str,
    ) -> Result<Box<dyn FSEFieldEncoder>, FSERecordEncoderMetadataError> {
        let encoder: Box<dyn FSEFieldEncoder> = match self {
            Self::Integer => Box::new(IntegerEncoder),
            Self::Float => Box::new(FloatEncoder),
            Self::Boolean => Box::new(BooleanEncoder),
            Self::TimestampMillis => Box::new(TimestampMillisEncoder),
            Self::CategoryDictionary { categories } => Box::new(
                CategoricalDictionaryEncoder::try_new(categories.clone()).map_err(|source| {
                    FSERecordEncoderMetadataError::CategoryDictionary {
                        field: field_index,
                        name: field_name.to_string(),
                        source,
                    }
                })?,
            ),
        };

        Ok(encoder)
    }
}

/// Metadata required to rebuild a composed record encoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FSERecordEncoderMetadata {
    fields: Vec<FSEFieldEncoderMetadata>,
}

impl FSERecordEncoderMetadata {
    /// Creates record encoder metadata from field metadata records.
    pub fn new(fields: Vec<FSEFieldEncoderMetadata>) -> Self {
        Self { fields }
    }

    /// Derives record encoder metadata from a typed record batch.
    pub fn from_batch(batch: &FSERecordBatch) -> Result<Self, ComposedRecordEncoderFromBatchError> {
        validate_no_null_values(batch)?;

        let fields = batch
            .schema()
            .fields()
            .iter()
            .enumerate()
            .map(|(field_index, field)| field_metadata_from_batch(batch, field_index, &field.name))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { fields })
    }

    /// Returns field metadata records in schema order.
    pub fn fields(&self) -> &[FSEFieldEncoderMetadata] {
        &self.fields
    }

    /// Rebuilds a composed record encoder for a schema.
    pub fn to_record_encoder(
        &self,
        schema: &FSESchema,
    ) -> Result<ComposedRecordEncoder, FSERecordEncoderMetadataError> {
        if self.fields.len() != schema.len() {
            return Err(FSERecordEncoderMetadataError::FieldCountMismatch {
                metadata_count: self.fields.len(),
                field_count: schema.len(),
            });
        }

        let mut field_encoders = Vec::with_capacity(self.fields.len());

        for (field_index, (metadata, field)) in self.fields.iter().zip(schema.fields()).enumerate()
        {
            let actual = metadata.field_type();

            if actual != field.field_type {
                return Err(FSERecordEncoderMetadataError::FieldTypeMismatch {
                    field: field_index,
                    name: field.name.clone(),
                    expected: field.field_type,
                    actual,
                });
            }

            field_encoders.push(metadata.to_field_encoder(field_index, &field.name)?);
        }

        ComposedRecordEncoder::try_new(schema, field_encoders).map_err(Into::into)
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

fn field_metadata_from_batch(
    batch: &FSERecordBatch,
    field_index: usize,
    field_name: &str,
) -> Result<FSEFieldEncoderMetadata, ComposedRecordEncoderFromBatchError> {
    let field = &batch.schema().fields()[field_index];

    match field.field_type {
        FSEFieldType::Integer => Ok(FSEFieldEncoderMetadata::Integer),
        FSEFieldType::Float => Ok(FSEFieldEncoderMetadata::Float),
        FSEFieldType::Boolean => Ok(FSEFieldEncoderMetadata::Boolean),
        FSEFieldType::TimestampMillis => Ok(FSEFieldEncoderMetadata::TimestampMillis),
        FSEFieldType::Category => category_metadata_from_batch(batch, field_index, field_name),
        FSEFieldType::Text => Err(ComposedRecordEncoderFromBatchError::UnsupportedFieldType {
            field: field_index,
            name: field_name.to_string(),
            field_type: field.field_type,
        }),
    }
}

fn category_metadata_from_batch(
    batch: &FSERecordBatch,
    field_index: usize,
    field_name: &str,
) -> Result<FSEFieldEncoderMetadata, ComposedRecordEncoderFromBatchError> {
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

    CategoricalDictionaryEncoder::try_new(categories.clone()).map_err(|source| {
        ComposedRecordEncoderFromBatchError::CategoryDictionary {
            field: field_index,
            name: field_name.to_string(),
            source,
        }
    })?;

    Ok(FSEFieldEncoderMetadata::CategoryDictionary { categories })
}
