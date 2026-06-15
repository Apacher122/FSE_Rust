use crate::data::{FSEField, FSEFieldType, FSERecord, FSESchema, FSEValue};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, ComposedRecordEncoderError,
    ComposedRecordEncoderFromBatchError, FSEEncodingError, FSERecordEncoder, FloatEncoder,
    IntegerEncoder,
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
fn composed_record_encoder_derives_encoder_from_record_batch() {
    let schema = crime_schema();
    let batch = crate::data::FSERecordBatch::new(
        schema.clone(),
        vec![crate::data::RowId::new(10), crate::data::RowId::new(11)],
        vec![
            crime_record(&schema, 42, 41.881, "open"),
            crime_record(&schema, 43, 41.882, "closed"),
        ],
    );

    let encoder = ComposedRecordEncoder::try_from_batch(&batch)
        .expect("valid batch should derive record encoder");
    let coordinates = encoder
        .encode_record(&crime_record(&schema, 44, 41.883, "closed"))
        .expect("known category should encode");

    assert_eq!(encoder.field_encoder_count(), 3);
    assert_eq!(encoder.output_dimensions(), 3);
    assert_eq!(coordinates.values(), &[44.0, 41.883, 1.0]);
}

#[test]
fn composed_record_encoder_derived_dictionary_uses_observed_category_order() {
    let schema = crime_schema();
    let batch = crate::data::FSERecordBatch::new(
        schema.clone(),
        vec![
            crate::data::RowId::new(10),
            crate::data::RowId::new(11),
            crate::data::RowId::new(12),
        ],
        vec![
            crime_record(&schema, 42, 41.881, "pending"),
            crime_record(&schema, 43, 41.882, "open"),
            crime_record(&schema, 44, 41.883, "pending"),
        ],
    );

    let encoder = ComposedRecordEncoder::try_from_batch(&batch)
        .expect("valid batch should derive record encoder");

    assert_eq!(
        encoder
            .encode_record(&crime_record(&schema, 45, 41.884, "pending"))
            .unwrap()
            .values(),
        &[45.0, 41.884, 0.0]
    );
    assert_eq!(
        encoder
            .encode_record(&crime_record(&schema, 46, 41.885, "open"))
            .unwrap()
            .values(),
        &[46.0, 41.885, 1.0]
    );
}

#[test]
fn composed_record_encoder_derivation_reports_unsupported_text_field() {
    let schema = FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("description", FSEFieldType::Text, false),
    ]);
    let batch = crate::data::FSERecordBatch::new(
        schema.clone(),
        vec![crate::data::RowId::new(10)],
        vec![FSERecord::new(
            vec![
                FSEValue::Integer(42),
                FSEValue::Text("burglary".to_string()),
            ],
            &schema,
        )],
    );

    let error = match ComposedRecordEncoder::try_from_batch(&batch) {
        Ok(_) => panic!("text field should not derive an encoder"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        ComposedRecordEncoderFromBatchError::UnsupportedFieldType {
            field: 1,
            name: "description".to_string(),
            field_type: FSEFieldType::Text,
        }
    );
    assert_eq!(
        error.to_string(),
        "field 'description' with type Text has no derived encoder"
    );
}

#[test]
fn composed_record_encoder_derivation_reports_null_field_value() {
    let schema = nullable_crime_schema();
    let batch = crate::data::FSERecordBatch::new(
        schema.clone(),
        vec![crate::data::RowId::new(10)],
        vec![FSERecord::new(
            vec![
                FSEValue::Integer(42),
                FSEValue::Float(41.881),
                FSEValue::Null,
            ],
            &schema,
        )],
    );

    let error = match ComposedRecordEncoder::try_from_batch(&batch) {
        Ok(_) => panic!("null field should not derive an encoder"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        ComposedRecordEncoderFromBatchError::NullFieldValue {
            record: 0,
            field: 2,
            name: "status".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "record 0 field 'status' is null and has no derived encoder"
    );
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
