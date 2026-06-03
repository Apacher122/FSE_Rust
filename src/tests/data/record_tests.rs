use crate::data::{FSEField, FSEFieldType, FSERecord, FSERecordError, FSESchema, FSEValue};

#[test]
fn record_accepts_values_matching_schema() {
    let schema = crime_schema();
    let record = FSERecord::new(
        vec![
            FSEValue::Integer(42),
            FSEValue::Text("burglary".to_string()),
            FSEValue::Float(41.881),
            FSEValue::Null,
        ],
        &schema,
    );

    assert_eq!(record.len(), 4);
    assert!(!record.is_empty());
    assert_eq!(record.value(0), Some(&FSEValue::Integer(42)));
    assert_eq!(
        record.value_named(&schema, "category"),
        Some(&FSEValue::Text("burglary".to_string()))
    );
    assert_eq!(record.value_named(&schema, "missing"), None);
}

#[test]
fn checked_record_reports_field_count_mismatch() {
    let schema = crime_schema();

    let error = FSERecord::try_new(vec![FSEValue::Integer(42)], &schema)
        .expect_err("record width mismatch should be rejected");

    assert_eq!(
        error,
        FSERecordError::FieldCountMismatch {
            value_count: 1,
            field_count: 4,
        }
    );
    assert_eq!(
        error.to_string(),
        "record has 1 values but schema requires 4"
    );
}

#[test]
fn checked_record_reports_null_for_non_nullable_field() {
    let schema = crime_schema();

    let error = FSERecord::try_new(
        vec![
            FSEValue::Integer(42),
            FSEValue::Null,
            FSEValue::Float(41.881),
            FSEValue::Null,
        ],
        &schema,
    )
    .expect_err("null in non-nullable field should be rejected");

    assert_eq!(
        error,
        FSERecordError::NullNotAllowed {
            field: 1,
            name: "category".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "field 'category' does not allow null values"
    );
}

#[test]
fn checked_record_allows_null_for_nullable_field() {
    let schema = crime_schema();
    let record = FSERecord::try_new(
        vec![
            FSEValue::Integer(42),
            FSEValue::Text("burglary".to_string()),
            FSEValue::Float(41.881),
            FSEValue::Null,
        ],
        &schema,
    )
    .expect("nullable field should accept null value");

    assert_eq!(record.value(3), Some(&FSEValue::Null));
}

#[test]
fn checked_record_reports_field_type_mismatch() {
    let schema = crime_schema();

    let error = FSERecord::try_new(
        vec![
            FSEValue::Text("42".to_string()),
            FSEValue::Text("burglary".to_string()),
            FSEValue::Float(41.881),
            FSEValue::Null,
        ],
        &schema,
    )
    .expect_err("field type mismatch should be rejected");

    assert_eq!(
        error,
        FSERecordError::FieldTypeMismatch {
            field: 0,
            name: "case_id".to_string(),
            expected: FSEFieldType::Integer,
            actual: FSEFieldType::Text,
        }
    );
    assert_eq!(
        error.to_string(),
        "field 'case_id' expected Integer but found Text"
    );
}

#[test]
#[should_panic(expected = "record has 1 values but schema requires 4")]
fn record_rejects_field_count_mismatch() {
    let schema = crime_schema();

    let _record = FSERecord::new(vec![FSEValue::Integer(42)], &schema);
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("category", FSEFieldType::Text, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("notes", FSEFieldType::Text, true),
    ])
}
