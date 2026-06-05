use crate::data::{
    FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError, FSESchema, FSEValue,
    RowId,
};

#[test]
fn record_batch_accepts_unique_row_ids_and_records() {
    let schema = crime_schema();
    let records = vec![
        crime_record(1, "burglary", &schema),
        crime_record(2, "assault", &schema),
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
    let batch = FSERecordBatch::new(crime_schema(), Vec::new(), Vec::new());

    assert_eq!(batch.len(), 0);
    assert!(batch.is_empty());
    assert!(batch.row_ids().is_empty());
    assert!(batch.records().is_empty());
}

#[test]
fn checked_record_batch_reports_row_id_count_mismatch() {
    let schema = crime_schema();
    let records = vec![crime_record(1, "burglary", &schema)];

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
    let schema = crime_schema();
    let records = vec![
        crime_record(1, "burglary", &schema),
        crime_record(2, "assault", &schema),
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
#[should_panic(expected = "record batch has 0 row ids but 1 records")]
fn record_batch_rejects_row_id_count_mismatch() {
    let schema = crime_schema();
    let records = vec![crime_record(1, "burglary", &schema)];

    let _batch = FSERecordBatch::new(schema, Vec::new(), records);
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("category", FSEFieldType::Text, false),
    ])
}

fn crime_record(case_id: i64, category: &str, schema: &FSESchema) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(case_id),
            FSEValue::Text(category.to_string()),
        ],
        schema,
    )
}
