//! Archive snapshots for typed FSE record batches.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::data::{
    FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError, FSERecordError,
    FSESchema, FSESchemaError, FSEValue, RowId,
};

/// Error returned when typed record batch archive validation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedRecordBatchArchiveSnapshotError {
    /// The archived schema is invalid.
    Schema(FSESchemaError),

    /// An archived record is invalid for the archived schema.
    Record {
        /// Record position in the archive.
        row_index: usize,

        /// Record validation error.
        source: FSERecordError,
    },

    /// The archived row identifiers and records do not form a valid batch.
    Batch(FSERecordBatchError),
}

impl fmt::Display for FSETypedRecordBatchArchiveSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(error) => error.fmt(formatter),
            Self::Record { source, .. } => source.fmt(formatter),
            Self::Batch(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETypedRecordBatchArchiveSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Schema(error) => Some(error),
            Self::Record { source, .. } => Some(source),
            Self::Batch(error) => Some(error),
        }
    }
}

impl From<FSESchemaError> for FSETypedRecordBatchArchiveSnapshotError {
    fn from(error: FSESchemaError) -> Self {
        Self::Schema(error)
    }
}

impl From<FSERecordBatchError> for FSETypedRecordBatchArchiveSnapshotError {
    fn from(error: FSERecordBatchError) -> Self {
        Self::Batch(error)
    }
}

/// Serializable field type tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FSEFieldTypeArchiveTag {
    /// Signed 64-bit integer field.
    Integer,

    /// 64-bit floating point field.
    Float,

    /// UTF-8 string field.
    Text,

    /// Boolean field.
    Boolean,

    /// Timestamp field represented as milliseconds since the Unix epoch.
    TimestampMillis,

    /// Categorical field represented by stable labels.
    Category,
}

impl FSEFieldTypeArchiveTag {
    fn from_field_type(field_type: FSEFieldType) -> Self {
        match field_type {
            FSEFieldType::Integer => Self::Integer,
            FSEFieldType::Float => Self::Float,
            FSEFieldType::Text => Self::Text,
            FSEFieldType::Boolean => Self::Boolean,
            FSEFieldType::TimestampMillis => Self::TimestampMillis,
            FSEFieldType::Category => Self::Category,
        }
    }

    fn to_field_type(self) -> FSEFieldType {
        match self {
            Self::Integer => FSEFieldType::Integer,
            Self::Float => FSEFieldType::Float,
            Self::Text => FSEFieldType::Text,
            Self::Boolean => FSEFieldType::Boolean,
            Self::TimestampMillis => FSEFieldType::TimestampMillis,
            Self::Category => FSEFieldType::Category,
        }
    }
}

/// Serializable field metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FSEFieldArchiveRecord {
    /// Stable field name.
    pub name: String,

    /// Logical field type.
    pub field_type: FSEFieldTypeArchiveTag,

    /// Whether the field accepts null values.
    pub nullable: bool,
}

impl FSEFieldArchiveRecord {
    /// Creates archive field metadata from runtime field metadata.
    pub fn from_field(field: &FSEField) -> Self {
        Self {
            name: field.name.clone(),
            field_type: FSEFieldTypeArchiveTag::from_field_type(field.field_type),
            nullable: field.nullable,
        }
    }

    fn to_field(&self) -> FSEField {
        FSEField::new(
            self.name.clone(),
            self.field_type.to_field_type(),
            self.nullable,
        )
    }
}

/// Serializable typed value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FSEValueArchiveRecord {
    /// Signed 64-bit integer value.
    Integer(i64),

    /// 64-bit floating point value.
    Float(f64),

    /// UTF-8 string value.
    Text(String),

    /// Boolean value.
    Boolean(bool),

    /// Timestamp value represented as milliseconds since the Unix epoch.
    TimestampMillis(i64),

    /// Categorical value represented by a stable label.
    Category(String),

    /// Missing value.
    Null,
}

impl FSEValueArchiveRecord {
    fn from_value(value: &FSEValue) -> Self {
        match value {
            FSEValue::Integer(value) => Self::Integer(*value),
            FSEValue::Float(value) => Self::Float(*value),
            FSEValue::Text(value) => Self::Text(value.clone()),
            FSEValue::Boolean(value) => Self::Boolean(*value),
            FSEValue::TimestampMillis(value) => Self::TimestampMillis(*value),
            FSEValue::Category(value) => Self::Category(value.clone()),
            FSEValue::Null => Self::Null,
        }
    }

    fn to_value(&self) -> FSEValue {
        match self {
            Self::Integer(value) => FSEValue::Integer(*value),
            Self::Float(value) => FSEValue::Float(*value),
            Self::Text(value) => FSEValue::Text(value.clone()),
            Self::Boolean(value) => FSEValue::Boolean(*value),
            Self::TimestampMillis(value) => FSEValue::TimestampMillis(*value),
            Self::Category(value) => FSEValue::Category(value.clone()),
            Self::Null => FSEValue::Null,
        }
    }
}

/// Serializable typed record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FSERecordArchiveRecord {
    /// Values stored in schema order.
    pub values: Vec<FSEValueArchiveRecord>,
}

impl FSERecordArchiveRecord {
    /// Creates an archive record from a runtime typed record.
    pub fn from_record(record: &FSERecord) -> Self {
        Self {
            values: record
                .values()
                .iter()
                .map(FSEValueArchiveRecord::from_value)
                .collect(),
        }
    }

    fn to_record(
        &self,
        schema: &FSESchema,
        row_index: usize,
    ) -> Result<FSERecord, FSETypedRecordBatchArchiveSnapshotError> {
        let values = self
            .values
            .iter()
            .map(FSEValueArchiveRecord::to_value)
            .collect();

        FSERecord::try_new(values, schema)
            .map_err(|source| FSETypedRecordBatchArchiveSnapshotError::Record { row_index, source })
    }
}

/// Serializable snapshot of a typed record batch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FSERecordBatchArchiveSnapshot {
    /// Schema fields in runtime order.
    pub schema_fields: Vec<FSEFieldArchiveRecord>,

    /// Row identifiers in record order.
    pub row_ids: Vec<u64>,

    /// Typed records in row identifier order.
    pub records: Vec<FSERecordArchiveRecord>,
}

impl FSERecordBatchArchiveSnapshot {
    /// Creates an archive snapshot from a runtime typed record batch.
    pub fn from_record_batch(batch: &FSERecordBatch) -> Self {
        Self {
            schema_fields: batch
                .schema()
                .fields()
                .iter()
                .map(FSEFieldArchiveRecord::from_field)
                .collect(),
            row_ids: batch
                .row_ids()
                .iter()
                .map(|row_id| row_id.value())
                .collect(),
            records: batch
                .records()
                .iter()
                .map(FSERecordArchiveRecord::from_record)
                .collect(),
        }
    }

    /// Validates the typed record batch archive snapshot.
    pub fn validate(&self) -> Result<(), FSETypedRecordBatchArchiveSnapshotError> {
        let _batch = self.to_record_batch()?;

        Ok(())
    }

    /// Rebuilds a runtime typed record batch from the archive snapshot.
    pub fn to_record_batch(
        &self,
    ) -> Result<FSERecordBatch, FSETypedRecordBatchArchiveSnapshotError> {
        let schema = self.to_schema()?;
        let row_ids = self
            .row_ids
            .iter()
            .map(|row_id| RowId::new(*row_id))
            .collect();
        let records = self
            .records
            .iter()
            .enumerate()
            .map(|(row_index, record)| record.to_record(&schema, row_index))
            .collect::<Result<Vec<_>, _>>()?;

        FSERecordBatch::try_new(schema, row_ids, records).map_err(Into::into)
    }

    fn to_schema(&self) -> Result<FSESchema, FSETypedRecordBatchArchiveSnapshotError> {
        let fields = self
            .schema_fields
            .iter()
            .map(FSEFieldArchiveRecord::to_field)
            .collect();

        FSESchema::try_new(fields).map_err(Into::into)
    }
}
