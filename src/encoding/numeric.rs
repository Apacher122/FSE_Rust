//! Numeric field encoders.

use crate::data::{FSEFieldType, FSEValue};

use super::{EncodedCoordinates, FSEEncodingError, FSEFieldEncoder};

/// Encoder for signed integer fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IntegerEncoder;

impl FSEFieldEncoder for IntegerEncoder {
    fn field_type(&self) -> FSEFieldType {
        FSEFieldType::Integer
    }

    fn output_dimensions(&self) -> usize {
        1
    }

    fn encode_value(&self, value: &FSEValue) -> Result<EncodedCoordinates, FSEEncodingError> {
        match value {
            FSEValue::Integer(value) => Ok(EncodedCoordinates::new(vec![*value as f32])),
            FSEValue::Null => Err(FSEEncodingError::NullValue),
            other => type_mismatch(FSEFieldType::Integer, other),
        }
    }
}

/// Encoder for floating point fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FloatEncoder;

impl FSEFieldEncoder for FloatEncoder {
    fn field_type(&self) -> FSEFieldType {
        FSEFieldType::Float
    }

    fn output_dimensions(&self) -> usize {
        1
    }

    fn encode_value(&self, value: &FSEValue) -> Result<EncodedCoordinates, FSEEncodingError> {
        match value {
            FSEValue::Float(value) if value.is_finite() => {
                Ok(EncodedCoordinates::new(vec![*value as f32]))
            }
            FSEValue::Float(_) => Err(FSEEncodingError::UnsupportedValue {
                reason: "float encoder requires finite values".to_string(),
            }),
            FSEValue::Null => Err(FSEEncodingError::NullValue),
            other => type_mismatch(FSEFieldType::Float, other),
        }
    }
}

/// Encoder for timestamp fields represented as milliseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimestampMillisEncoder;

impl FSEFieldEncoder for TimestampMillisEncoder {
    fn field_type(&self) -> FSEFieldType {
        FSEFieldType::TimestampMillis
    }

    fn output_dimensions(&self) -> usize {
        1
    }

    fn encode_value(&self, value: &FSEValue) -> Result<EncodedCoordinates, FSEEncodingError> {
        match value {
            FSEValue::TimestampMillis(value) => Ok(EncodedCoordinates::new(vec![*value as f32])),
            FSEValue::Null => Err(FSEEncodingError::NullValue),
            other => type_mismatch(FSEFieldType::TimestampMillis, other),
        }
    }
}

/// Encoder for boolean fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BooleanEncoder;

impl FSEFieldEncoder for BooleanEncoder {
    fn field_type(&self) -> FSEFieldType {
        FSEFieldType::Boolean
    }

    fn output_dimensions(&self) -> usize {
        1
    }

    fn encode_value(&self, value: &FSEValue) -> Result<EncodedCoordinates, FSEEncodingError> {
        match value {
            FSEValue::Boolean(false) => Ok(EncodedCoordinates::new(vec![0.0])),
            FSEValue::Boolean(true) => Ok(EncodedCoordinates::new(vec![1.0])),
            FSEValue::Null => Err(FSEEncodingError::NullValue),
            other => type_mismatch(FSEFieldType::Boolean, other),
        }
    }
}

fn type_mismatch(
    expected: FSEFieldType,
    value: &FSEValue,
) -> Result<EncodedCoordinates, FSEEncodingError> {
    Err(FSEEncodingError::FieldTypeMismatch {
        expected,
        actual: value
            .field_type()
            .expect("non-null value should have field type"),
    })
}
