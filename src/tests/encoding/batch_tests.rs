use crate::data::{FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema, FSEValue, RowId};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, EncodedCoordinates, FSEEncodingError,
    FSERecordBatchEncodingError, FSERecordEncoder, FloatEncoder, IntegerEncoder,
    encode_record_batch,
};

#[test]
fn record_batch_encoder_preserves_row_ids_and_encodes_vectors() {
    let schema = crime_schema();
    let batch = crime_batch(&schema);
    let encoder = crime_encoder(&schema);

    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");

    assert_eq!(encoded.len(), 2);
    assert_eq!(encoded.dimensions(), 3);
    assert_eq!(encoded.row_ids(), &[RowId::new(10), RowId::new(11)]);
    assert_eq!(encoded.vectors()[0].values, vec![42.0, 41.881, 0.0]);
    assert_eq!(encoded.vectors()[1].values, vec![43.0, 41.882, 1.0]);
}

#[test]
fn record_batch_encoder_returns_record_context_on_encoding_error() {
    let schema = crime_schema();
    let batch = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10), RowId::new(11)],
        vec![
            crime_record(&schema, 42, 41.881, "open"),
            crime_record(&schema, 43, 41.882, "pending"),
        ],
    );
    let encoder = crime_encoder(&schema);

    let error = encode_record_batch(&batch, &encoder)
        .expect_err("unknown category should fail batch encoding");

    assert_eq!(
        error,
        FSERecordBatchEncodingError::RecordEncoding {
            record: 1,
            row_id: RowId::new(11),
            source: FSEEncodingError::UnsupportedValue {
                reason: "category 'pending' is not in dictionary".to_string(),
            },
        }
    );
    assert_eq!(
        error.to_string(),
        "record 1 with row id 11 could not be encoded"
    );
}

#[test]
fn record_batch_encoder_rejects_dimension_mismatch() {
    let schema = crime_schema();
    let batch = crime_batch(&schema);
    let encoder = MismatchedRecordEncoder;

    let error = encode_record_batch(&batch, &encoder)
        .expect_err("coordinate count mismatch should fail batch encoding");

    assert_eq!(
        error,
        FSERecordBatchEncodingError::DimensionMismatch {
            record: 0,
            row_id: RowId::new(10),
            expected: 3,
            actual: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "record 0 with row id 10 produced 2 coordinates but encoder requires 3"
    );
}

#[test]
fn encoded_record_batch_constructor_rejects_inconsistent_vector_dimensions() {
    let error = crate::encoding::EncodedRecordBatch::try_new(
        vec![RowId::new(10), RowId::new(11)],
        vec![
            crate::math::Vector::new(vec![1.0, 2.0]),
            crate::math::Vector::new(vec![1.0, 2.0, 3.0]),
        ],
    )
    .expect_err("inconsistent vector dimensions should be rejected");

    assert_eq!(
        error,
        FSERecordBatchEncodingError::DimensionMismatch {
            record: 1,
            row_id: RowId::new(11),
            expected: 2,
            actual: 3,
        }
    );
}

#[test]
#[should_panic(expected = "record 1 with row id 11 produced 3 coordinates but encoder requires 2")]
fn encoded_record_batch_constructor_panics_on_invalid_dimensions() {
    let _encoded = crate::encoding::EncodedRecordBatch::new(
        vec![RowId::new(10), RowId::new(11)],
        vec![
            crate::math::Vector::new(vec![1.0, 2.0]),
            crate::math::Vector::new(vec![1.0, 2.0, 3.0]),
        ],
    );
}

struct MismatchedRecordEncoder;

impl FSERecordEncoder for MismatchedRecordEncoder {
    fn output_dimensions(&self) -> usize {
        3
    }

    fn encode_record(&self, _record: &FSERecord) -> Result<EncodedCoordinates, FSEEncodingError> {
        Ok(EncodedCoordinates::new(vec![1.0, 2.0]))
    }
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("status", FSEFieldType::Category, false),
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

fn crime_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10), RowId::new(11)],
        vec![
            crime_record(schema, 42, 41.881, "open"),
            crime_record(schema, 43, 41.882, "closed"),
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
