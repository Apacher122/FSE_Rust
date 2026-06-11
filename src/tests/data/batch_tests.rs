use crate::data::{
    FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError, FSESchema, FSEValue,
    RowId,
};

#[test]
fn record_batch_accepts_unique_row_ids_and_records() {
    let schema = entity_schema();
    let records = vec![
        entity_record(1, "alpha", &schema),
        entity_record(2, "beta", &schema),
    ];
    let batch = FSERecordBatch::new(schema, vec![RowId::new(10), RowId::new(11)], records);

    assert_eq!(batch.len(), 2);
    assert!(!batch.is_empty());
    assert_eq!(batch.schema().len(), 2);
    assert_eq!(batch.row_ids(), &[RowId::new(10), RowId::new(11)]);
    assert_eq!(batch.row_index_for_row_id(RowId::new(10)), Some(0));
    assert_eq!(batch.row_index_for_row_id(RowId::new(11)), Some(1));
    assert_eq!(batch.row_index_for_row_id(RowId::new(12)), None);
    assert_eq!(
        batch
            .record_for_row_id(RowId::new(11))
            .expect("record should exist")
            .value(0),
        Some(&FSEValue::Integer(2))
    );
    assert_eq!(batch.record_for_row_id(RowId::new(12)), None);
}

#[test]
fn record_batch_accepts_empty_records_with_valid_schema() {
    let batch = FSERecordBatch::new(entity_schema(), Vec::new(), Vec::new());

    assert_eq!(batch.len(), 0);
    assert!(batch.is_empty());
    assert!(batch.row_ids().is_empty());
    assert!(batch.records().is_empty());
}

#[test]
fn checked_record_batch_reports_row_id_count_mismatch() {
    let schema = entity_schema();
    let records = vec![entity_record(1, "alpha", &schema)];

    let error = FSERecordBatch::try_new(schema, Vec::new(), records)
        .expect_err("row id count mismatch should be rejected");

    assert_eq!(
        error,
        FSERecordBatchError::RowIdCountMismatch {
            row_id_count: 0,
            record_count: 1,
        }
    );
    assert_eq!(
        error.to_string(),
        "record batch has 0 row ids but 1 records"
    );
}

#[test]
fn checked_record_batch_reports_duplicate_row_ids() {
    let schema = entity_schema();
    let records = vec![
        entity_record(1, "alpha", &schema),
        entity_record(2, "beta", &schema),
    ];

    let error = FSERecordBatch::try_new(schema, vec![RowId::new(10), RowId::new(10)], records)
        .expect_err("duplicate row ids should be rejected");

    assert_eq!(
        error,
        FSERecordBatchError::DuplicateRowId {
            row_id: RowId::new(10),
        }
    );
    assert_eq!(error.to_string(), "row id 10 appears more than once");
}

#[test]
fn checked_record_batch_append_preserves_base_then_appended_order() {
    let schema = entity_schema();
    let base = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10), RowId::new(11)],
        vec![
            entity_record(1, "alpha", &schema),
            entity_record(2, "beta", &schema),
        ],
    );
    let appended = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(12), RowId::new(13)],
        vec![
            entity_record(3, "gamma", &schema),
            entity_record(4, "delta", &schema),
        ],
    );

    let combined = base.try_append(&appended).unwrap();

    assert_eq!(combined.schema(), &schema);
    assert_eq!(
        combined.row_ids(),
        &[
            RowId::new(10),
            RowId::new(11),
            RowId::new(12),
            RowId::new(13)
        ]
    );
    assert_eq!(combined.len(), 4);
    assert_eq!(
        combined
            .record_for_row_id(RowId::new(13))
            .expect("appended record should exist")
            .value(0),
        Some(&FSEValue::Integer(4))
    );
    assert_eq!(combined.row_index_for_row_id(RowId::new(12)), Some(2));
    assert_eq!(combined.row_index_for_row_id(RowId::new(13)), Some(3));
}

#[test]
fn checked_record_batch_append_reports_empty_append_batch() {
    let schema = entity_schema();
    let base = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10)],
        vec![entity_record(1, "alpha", &schema)],
    );
    let appended = FSERecordBatch::new(schema, Vec::new(), Vec::new());

    assert_eq!(
        base.try_append(&appended),
        Err(FSERecordBatchError::EmptyAppendBatch)
    );
}

#[test]
fn checked_record_batch_append_reports_schema_mismatch() {
    let schema = entity_schema();
    let other_schema = FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
    ]);
    let base = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10)],
        vec![entity_record(1, "alpha", &schema)],
    );
    let appended = FSERecordBatch::new(
        other_schema.clone(),
        vec![RowId::new(11)],
        vec![FSERecord::new(
            vec![FSEValue::Integer(2), FSEValue::Float(12.5)],
            &other_schema,
        )],
    );

    assert_eq!(
        base.try_append(&appended),
        Err(FSERecordBatchError::SchemaMismatch)
    );
}

#[test]
fn checked_record_batch_append_reports_duplicate_row_ids() {
    let schema = entity_schema();
    let base = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10)],
        vec![entity_record(1, "alpha", &schema)],
    );
    let appended = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10)],
        vec![entity_record(2, "beta", &schema)],
    );

    assert_eq!(
        base.try_append(&appended),
        Err(FSERecordBatchError::DuplicateRowId {
            row_id: RowId::new(10)
        })
    );
}

#[test]
#[should_panic(expected = "record batch has 0 row ids but 1 records")]
fn record_batch_rejects_row_id_count_mismatch() {
    let schema = entity_schema();
    let records = vec![entity_record(1, "alpha", &schema)];

    let _batch = FSERecordBatch::new(schema, Vec::new(), records);
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("label", FSEFieldType::Text, false),
    ])
}

fn entity_record(entity_id: i64, label: &str, schema: &FSESchema) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(entity_id),
            FSEValue::Text(label.to_string()),
        ],
        schema,
    )
}
