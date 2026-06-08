use crate::data::{
    FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordError, FSESchema, FSEValue, RowId,
};
use crate::persistence::{
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveCodecError,
    FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
    decode_typed_record_batch_archive_snapshot, encode_typed_record_batch_archive_snapshot,
};

#[test]
fn typed_record_batch_archive_codec_round_trips_snapshot() {
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    let bytes = encode_typed_record_batch_archive_snapshot(&snapshot).unwrap();
    let decoded = decode_typed_record_batch_archive_snapshot(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_record_batch_archive_codec_methods_round_trip_snapshot() {
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSERecordBatchArchiveSnapshot::from_archive_bytes(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_record_batch_archive_codec_rebuilds_batch_after_decode() {
    let batch = sample_batch();
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSERecordBatchArchiveSnapshot::from_archive_bytes(&bytes).unwrap();

    assert_eq!(decoded.to_record_batch().unwrap(), batch);
}

#[test]
fn typed_record_batch_archive_codec_rejects_invalid_snapshot_on_encode() {
    let mut snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    snapshot.records[0].values[0] = FSEValueArchiveRecord::Text("1".to_string());

    assert_eq!(
        encode_typed_record_batch_archive_snapshot(&snapshot),
        Err(FSETypedRecordBatchArchiveCodecError::Snapshot(
            FSETypedRecordBatchArchiveSnapshotError::Record {
                row_index: 0,
                source: FSERecordError::FieldTypeMismatch {
                    field: 0,
                    name: "record_id".to_string(),
                    expected: FSEFieldType::Integer,
                    actual: FSEFieldType::Text,
                },
            },
        ))
    );
}

#[test]
fn typed_record_batch_archive_codec_rejects_truncated_bytes() {
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    bytes.pop();

    assert!(matches!(
        decode_typed_record_batch_archive_snapshot(&bytes),
        Err(FSETypedRecordBatchArchiveCodecError::UnexpectedEndOfArchive { .. })
    ));
}

#[test]
fn typed_record_batch_archive_codec_rejects_trailing_bytes() {
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    bytes.push(99);

    assert_eq!(
        decode_typed_record_batch_archive_snapshot(&bytes),
        Err(FSETypedRecordBatchArchiveCodecError::TrailingBytes { remaining: 1 })
    );
}

#[test]
fn typed_record_batch_archive_codec_rejects_invalid_field_type_tag() {
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    let field_type_offset = first_field_type_offset(&snapshot);
    bytes[field_type_offset] = 99;

    assert_eq!(
        decode_typed_record_batch_archive_snapshot(&bytes),
        Err(FSETypedRecordBatchArchiveCodecError::InvalidFieldTypeTag {
            field: "typed_batch.schema.field.field_type",
            value: 99,
        })
    );
}

#[test]
fn typed_record_batch_archive_codec_rejects_invalid_boolean() {
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    let nullable_offset = first_field_type_offset(&snapshot) + 1;
    bytes[nullable_offset] = 99;

    assert_eq!(
        decode_typed_record_batch_archive_snapshot(&bytes),
        Err(FSETypedRecordBatchArchiveCodecError::InvalidBoolean {
            field: "typed_batch.schema.field.nullable",
            value: 99,
        })
    );
}

#[test]
fn typed_record_batch_archive_codec_rejects_invalid_value_tag() {
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&sample_batch());
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    let value_tag_offset = first_value_tag_offset(&snapshot);
    bytes[value_tag_offset] = 99;

    assert_eq!(
        decode_typed_record_batch_archive_snapshot(&bytes),
        Err(FSETypedRecordBatchArchiveCodecError::InvalidValueTag {
            field: "typed_batch.record.value",
            value: 99,
        })
    );
}

fn sample_batch() -> FSERecordBatch {
    let schema = sample_schema();
    let records = vec![
        sample_record(
            1,
            12.5,
            "alpha",
            true,
            1_735_689_600_000,
            "open",
            FSEValue::Text("reviewed".to_string()),
            &schema,
        ),
        sample_record(
            2,
            24.0,
            "beta",
            false,
            1_735_776_000_000,
            "closed",
            FSEValue::Null,
            &schema,
        ),
    ];

    FSERecordBatch::new(schema, vec![RowId::new(100), RowId::new(101)], records)
}

fn sample_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("record_id", FSEFieldType::Integer, false),
        FSEField::new("amount", FSEFieldType::Float, false),
        FSEField::new("label", FSEFieldType::Text, false),
        FSEField::new("active", FSEFieldType::Boolean, false),
        FSEField::new("created_at", FSEFieldType::TimestampMillis, false),
        FSEField::new("status", FSEFieldType::Category, false),
        FSEField::new("notes", FSEFieldType::Text, true),
    ])
}

fn sample_record(
    record_id: i64,
    amount: f64,
    label: &str,
    active: bool,
    created_at: i64,
    status: &str,
    notes: FSEValue,
    schema: &FSESchema,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(record_id),
            FSEValue::Float(amount),
            FSEValue::Text(label.to_string()),
            FSEValue::Boolean(active),
            FSEValue::TimestampMillis(created_at),
            FSEValue::Category(status.to_string()),
            notes,
        ],
        schema,
    )
}

fn first_field_type_offset(snapshot: &FSERecordBatchArchiveSnapshot) -> usize {
    size_of_u64() + size_of_u64() + snapshot.schema_fields[0].name.len()
}

fn first_value_tag_offset(snapshot: &FSERecordBatchArchiveSnapshot) -> usize {
    let mut offset = size_of_u64();

    for field in &snapshot.schema_fields {
        offset += size_of_u64();
        offset += field.name.len();
        offset += 1;
        offset += 1;
    }

    offset += size_of_u64();
    offset += snapshot.row_ids.len() * size_of_u64();
    offset += size_of_u64();
    offset += size_of_u64();

    offset
}

fn size_of_u64() -> usize {
    std::mem::size_of::<u64>()
}
