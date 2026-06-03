//! FSE-native typed records.

use std::error::Error;
use std::fmt;

use super::{FSEFieldType, FSESchema, FSEValue};

/// Error returned when checked record construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERecordError {
    /// The number of values did not match the schema field count.
    FieldCountMismatch {
        /// Number of values provided by the record.
        value_count: usize,
        /// Number of fields required by the schema.
        field_count: usize,
    },

    /// A null value was provided for a non-nullable field.
    NullNotAllowed {
        /// Field index containing the invalid null.
        field: usize,
        /// Field name containing the invalid null.
        name: String,
    },

    /// A value type did not match the schema field type.
    FieldTypeMismatch {
        /// Field index containing the mismatched value.
        field: usize,
        /// Field name containing the mismatched value.
        name: String,
        /// Field type required by the schema.
        expected: FSEFieldType,
        /// Field type found in the value.
        actual: FSEFieldType,
    },
}

impl fmt::Display for FSERecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldCountMismatch {
                value_count,
                field_count,
            } => write!(
                formatter,
                "record has {value_count} values but schema requires {field_count}"
            ),
            Self::NullNotAllowed { name, .. } => {
                write!(formatter, "field '{name}' does not allow null values")
            }
            Self::FieldTypeMismatch {
                name,
                expected,
                actual,
                ..
            } => write!(
                formatter,
                "field '{name}' expected {expected:?} but found {actual:?}"
            ),
        }
    }
}

impl Error for FSERecordError {}

/// Typed row validated against an FSE schema.
///
/// # Runtime Role
///
/// `FSERecord` stores logical values before semantic encoding. Construction
/// validates field count, field type, and nullability against a schema.
#[derive(Clone, Debug, PartialEq)]
pub struct FSERecord {
    values: Vec<FSEValue>,
}

impl FSERecord {
    /// Creates a typed record from values and schema metadata.
    ///
    /// # Panics
    ///
    /// Panics when the record does not satisfy the schema.
    pub fn new(values: Vec<FSEValue>, schema: &FSESchema) -> Self {
        Self::try_new(values, schema).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a typed record and returns an error when validation fails.
    pub fn try_new(values: Vec<FSEValue>, schema: &FSESchema) -> Result<Self, FSERecordError> {
        if values.len() != schema.len() {
            return Err(FSERecordError::FieldCountMismatch {
                value_count: values.len(),
                field_count: schema.len(),
            });
        }

        for (field_index, (value, field)) in values.iter().zip(schema.fields()).enumerate() {
            match value.field_type() {
                Some(actual) if actual != field.field_type => {
                    return Err(FSERecordError::FieldTypeMismatch {
                        field: field_index,
                        name: field.name.clone(),
                        expected: field.field_type,
                        actual,
                    });
                }
                None if !field.nullable => {
                    return Err(FSERecordError::NullNotAllowed {
                        field: field_index,
                        name: field.name.clone(),
                    });
                }
                _ => {}
            }
        }

        Ok(Self { values })
    }

    /// Returns record values in schema order.
    pub fn values(&self) -> &[FSEValue] {
        &self.values
    }

    /// Returns the number of values in the record.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true when the record contains no values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the value at the given field index.
    pub fn value(&self, index: usize) -> Option<&FSEValue> {
        self.values.get(index)
    }

    /// Returns the value for the given schema field name.
    pub fn value_named<'a>(&'a self, schema: &FSESchema, name: &str) -> Option<&'a FSEValue> {
        let index = schema
            .fields()
            .iter()
            .position(|field| field.name == name)?;

        self.value(index)
    }
}
