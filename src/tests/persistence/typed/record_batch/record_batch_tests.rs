use crate::data::{
    FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError, FSERecordError,
    FSESchema, FSESchemaError, FSEValue, RowId,
};
use crate::persistence::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
};

#[test]
fn typed_record_batch_archive_snapshot_captures_schema_row_ids_and_records() {
    let batch = sample_batch();
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);

    assert_eq!(
        snapshot.schema_fields,
        vec![
            FSEFieldArchiveRecord {
                name: "record_id".to_string(),
                field_type: FSEFieldTypeArchiveTag::Integer,
                nullable: false,
            },
            FSEFieldArchiveRecord {
                name: "amount".to_string(),
                field_type: FSEFieldTypeArchiveTag::Float,
                nullable: false,
            },
            FSEFieldArchiveRecord {
                name: "label".to_string(),
                field_type: FSEFieldTypeArchiveTag::Text,
                nullable: false,
            },
            FSEFieldArchiveRecord {
                name: "active".to_string(),
                field_type: FSEFieldTypeArchiveTag::Boolean,
                nullable: false,
            },
            FSEFieldArchiveRecord {
                name: "created_at".to_string(),
                field_type: FSEFieldTypeArchiveTag::TimestampMillis,
                nullable: false,
            },
            FSEFieldArchiveRecord {
                name: "status".to_string(),
                field_type: FSEFieldTypeArchiveTag::Category,
                nullable: false,
            },
            FSEFieldArchiveRecord {
                name: "notes".to_string(),
                field_type: FSEFieldTypeArchiveTag::Text,
                nullable: true,
            },
        ]
    );
    assert_eq!(snapshot.row_ids, vec![100, 101]);
    assert_eq!(
        snapshot.records[1],
        FSERecordArchiveRecord {
            values: vec![
                FSEValueArchiveRecord::Integer(2),
                FSEValueArchiveRecord::Float(24.0),
                FSEValueArchiveRecord::Text("beta".to_string()),
                FSEValueArchiveRecord::Boolean(false),
                FSEValueArchiveRecord::TimestampMillis(1_735_776_000_000),
                FSEValueArchiveRecord::Category("closed".to_string()),
                FSEValueArchiveRecord::Null,
            ],
        }
    );
}

#[test]
fn typed_record_batch_archive_snapshot_rebuilds_batch() {
    let batch = sample_batch();
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    let rebuilt = snapshot.to_record_batch().unwrap();

    assert_eq!(rebuilt, batch);
    assert!(snapshot.validate().is_ok());
}

#[test]
fn typed_record_batch_archive_snapshot_reports_invalid_schema() {
    let batch = sample_batch();
    let mut snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    snapshot.schema_fields[1].name = "record_id".to_string();

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedRecordBatchArchiveSnapshotError::Schema(
            FSESchemaError::DuplicateFieldName {
                name: "record_id".to_string(),
            },
        ))
    );
}

#[test]
fn typed_record_batch_archive_snapshot_reports_invalid_record_value() {
    let batch = sample_batch();
    let mut snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    snapshot.records[0].values[0] = FSEValueArchiveRecord::Text("1".to_string());

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedRecordBatchArchiveSnapshotError::Record {
            row_index: 0,
            source: FSERecordError::FieldTypeMismatch {
                field: 0,
                name: "record_id".to_string(),
                expected: FSEFieldType::Integer,
                actual: FSEFieldType::Text,
            },
        })
    );
}

#[test]
fn typed_record_batch_archive_snapshot_reports_row_id_count_mismatch() {
    let batch = sample_batch();
    let mut snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    snapshot.row_ids.pop();

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedRecordBatchArchiveSnapshotError::Batch(
            FSERecordBatchError::RowIdCountMismatch {
                row_id_count: 1,
                record_count: 2,
            },
        ))
    );
}

#[test]
fn typed_record_batch_archive_snapshot_reports_duplicate_row_id() {
    let batch = sample_batch();
    let mut snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    snapshot.row_ids[1] = 100;

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedRecordBatchArchiveSnapshotError::Batch(
            FSERecordBatchError::DuplicateRowId {
                row_id: RowId::new(100),
            },
        ))
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
