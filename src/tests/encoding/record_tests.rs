use crate::data::{FSEField, FSEFieldType, FSERecord, FSESchema, FSEValue};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, ComposedRecordEncoderError,
    FSEEncodingError, FSERecordEncoder, FloatEncoder, IntegerEncoder,
};

#[test]
fn composed_record_encoder_encodes_record_fields_in_schema_order() {
    let schema = crime_schema();
    let encoder = crime_encoder(&schema);
    let record = crime_record(&schema, 42, 41.881, "open");

    let coordinates = encoder
        .encode_record(&record)
        .expect("valid record should encode");

    assert_eq!(encoder.field_encoder_count(), 3);
    assert_eq!(encoder.output_dimensions(), 3);
    assert_eq!(coordinates.values(), &[42.0, 41.881, 0.0]);
}

#[test]
fn composed_record_encoder_reports_encoder_count_mismatch() {
    let schema = crime_schema();

    let error = match ComposedRecordEncoder::try_new(&schema, vec![Box::new(IntegerEncoder)]) {
        Ok(_) => panic!("encoder count mismatch should be rejected"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        ComposedRecordEncoderError::EncoderCountMismatch {
            encoder_count: 1,
            field_count: 3,
        }
    );
    assert_eq!(
        error.to_string(),
        "record encoder has 1 field encoders but schema requires 3"
    );
}

#[test]
fn composed_record_encoder_reports_field_type_mismatch() {
    let schema = crime_schema();

    let error = match ComposedRecordEncoder::try_new(
        &schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(IntegerEncoder),
            Box::new(CategoricalDictionaryEncoder::new(vec![
                "open".to_string(),
                "closed".to_string(),
            ])),
        ],
    ) {
        Ok(_) => panic!("field type mismatch should be rejected"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        ComposedRecordEncoderError::FieldTypeMismatch {
            field: 1,
            name: "latitude".to_string(),
            expected: FSEFieldType::Float,
            actual: FSEFieldType::Integer,
        }
    );
    assert_eq!(
        error.to_string(),
        "field 'latitude' encoder expected Float but found Integer"
    );
}

#[test]
fn composed_record_encoder_propagates_unknown_category_error() {
    let schema = crime_schema();
    let encoder = crime_encoder(&schema);
    let record = crime_record(&schema, 42, 41.881, "pending");

    let error = encoder
        .encode_record(&record)
        .expect_err("unknown category should be rejected");

    assert_eq!(
        error,
        FSEEncodingError::UnsupportedValue {
            reason: "category 'pending' is not in dictionary".to_string(),
        }
    );
}

#[test]
fn composed_record_encoder_propagates_null_error() {
    let schema = nullable_crime_schema();
    let encoder = ComposedRecordEncoder::new(
        &schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(FloatEncoder),
            Box::new(CategoricalDictionaryEncoder::new(vec![
                "open".to_string(),
                "closed".to_string(),
            ])),
        ],
    );
    let record = FSERecord::new(
        vec![
            FSEValue::Integer(42),
            FSEValue::Float(41.881),
            FSEValue::Null,
        ],
        &schema,
    );

    let error = encoder
        .encode_record(&record)
        .expect_err("null category should be rejected by field encoder");

    assert_eq!(error, FSEEncodingError::NullValue);
}

#[test]
#[should_panic(expected = "record encoder has 1 field encoders but schema requires 3")]
fn composed_record_encoder_rejects_encoder_count_mismatch() {
    let schema = crime_schema();

    let _encoder = ComposedRecordEncoder::new(&schema, vec![Box::new(IntegerEncoder)]);
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("status", FSEFieldType::Category, false),
    ])
}

fn nullable_crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("status", FSEFieldType::Category, true),
    ])
}

fn crime_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
    ComposedRecordEncoder::new(
        schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(FloatEncoder),
            Box::new(CategoricalDictionaryEncoder::new(vec![
                "open".to_string(),
                "closed".to_string(),
            ])),
        ],
    )
}

fn crime_record(schema: &FSESchema, case_id: i64, latitude: f64, status: &str) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(case_id),
            FSEValue::Float(latitude),
            FSEValue::Category(status.to_string()),
        ],
        schema,
    )
}
