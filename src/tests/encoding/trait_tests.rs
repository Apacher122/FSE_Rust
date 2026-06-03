use crate::data::{FSEField, FSEFieldType, FSERecord, FSESchema, FSEValue};
use crate::encoding::{EncodedCoordinates, FSEEncodingError, FSEFieldEncoder, FSERecordEncoder};

#[test]
fn encoded_coordinates_report_shape() {
    let coordinates = EncodedCoordinates::new(vec![1.0, 2.0]);

    assert_eq!(coordinates.values(), &[1.0, 2.0]);
    assert_eq!(coordinates.dimensions(), 2);
    assert!(!coordinates.is_empty());
}

#[test]
fn field_encoder_contract_encodes_matching_values() {
    let encoder = IntegerIdentityEncoder;

    let coordinates = encoder
        .encode_value(&FSEValue::Integer(42))
        .expect("integer value should encode");

    assert_eq!(encoder.field_type(), FSEFieldType::Integer);
    assert_eq!(encoder.output_dimensions(), 1);
    assert_eq!(coordinates.values(), &[42.0]);
}

#[test]
fn field_encoder_contract_reports_null_values() {
    let encoder = IntegerIdentityEncoder;

    let error = encoder
        .encode_value(&FSEValue::Null)
        .expect_err("null should be rejected by test encoder");

    assert_eq!(error, FSEEncodingError::NullValue);
    assert_eq!(error.to_string(), "encoder cannot encode null value");
}

#[test]
fn field_encoder_contract_reports_type_mismatch() {
    let encoder = IntegerIdentityEncoder;

    let error = encoder
        .encode_value(&FSEValue::Text("42".to_string()))
        .expect_err("text should be rejected by integer encoder");

    assert_eq!(
        error,
        FSEEncodingError::FieldTypeMismatch {
            expected: FSEFieldType::Integer,
            actual: FSEFieldType::Text,
        }
    );
    assert_eq!(error.to_string(), "encoder expected Integer but found Text");
}

#[test]
fn record_encoder_contract_encodes_valid_record() {
    let schema = FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("priority", FSEFieldType::Integer, false),
    ]);
    let record = FSERecord::new(vec![FSEValue::Integer(10), FSEValue::Integer(3)], &schema);
    let encoder = TwoIntegerRecordEncoder;

    let coordinates = encoder
        .encode_record(&record)
        .expect("integer record should encode");

    assert_eq!(encoder.output_dimensions(), 2);
    assert_eq!(coordinates.values(), &[10.0, 3.0]);
}

struct IntegerIdentityEncoder;

impl FSEFieldEncoder for IntegerIdentityEncoder {
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
            other => Err(FSEEncodingError::FieldTypeMismatch {
                expected: FSEFieldType::Integer,
                actual: other
                    .field_type()
                    .expect("non-null value should have field type"),
            }),
        }
    }
}

struct TwoIntegerRecordEncoder;

impl FSERecordEncoder for TwoIntegerRecordEncoder {
    fn output_dimensions(&self) -> usize {
        2
    }

    fn encode_record(&self, record: &FSERecord) -> Result<EncodedCoordinates, FSEEncodingError> {
        let first = IntegerIdentityEncoder.encode_value(record.value(0).ok_or_else(|| {
            FSEEncodingError::UnsupportedValue {
                reason: "record is missing first integer field".to_string(),
            }
        })?)?;
        let second = IntegerIdentityEncoder.encode_value(record.value(1).ok_or_else(|| {
            FSEEncodingError::UnsupportedValue {
                reason: "record is missing second integer field".to_string(),
            }
        })?)?;

        Ok(EncodedCoordinates::new(vec![
            first.values()[0],
            second.values()[0],
        ]))
    }
}
