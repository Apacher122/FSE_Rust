use crate::data::{FSEField, FSEFieldType, FSESchema, FSESchemaError, FSEValue};

#[test]
fn fse_value_reports_non_null_field_type() {
    assert_eq!(
        FSEValue::Integer(7).field_type(),
        Some(FSEFieldType::Integer)
    );
    assert_eq!(FSEValue::Float(1.5).field_type(), Some(FSEFieldType::Float));
    assert_eq!(
        FSEValue::Text("alpha".to_string()).field_type(),
        Some(FSEFieldType::Text)
    );
    assert_eq!(
        FSEValue::Boolean(true).field_type(),
        Some(FSEFieldType::Boolean)
    );
}

#[test]
fn fse_value_reports_null_without_field_type() {
    let value = FSEValue::Null;

    assert_eq!(value.field_type(), None);
    assert!(value.is_null());
}

#[test]
fn schema_accepts_unique_named_fields() {
    let schema = FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("category", FSEFieldType::Text, false),
        FSEField::new("latitude", FSEFieldType::Float, true),
    ]);

    assert_eq!(schema.len(), 3);
    assert!(!schema.is_empty());
    assert_eq!(schema.fields()[0].name, "case_id");
    assert_eq!(
        schema.field(1).expect("field should exist").name,
        "category"
    );
    assert_eq!(
        schema
            .field_named("latitude")
            .expect("field should exist")
            .field_type,
        FSEFieldType::Float
    );
}

#[test]
fn checked_schema_reports_empty_fields() {
    let error = FSESchema::try_new(Vec::new()).expect_err("empty schema should be rejected");

    assert_eq!(error, FSESchemaError::EmptyFields);
    assert_eq!(error.to_string(), "schema must contain at least one field");
}

#[test]
fn checked_schema_reports_empty_field_name() {
    let error = FSESchema::try_new(vec![FSEField::new("", FSEFieldType::Text, false)])
        .expect_err("empty field name should be rejected");

    assert_eq!(error, FSESchemaError::EmptyFieldName { index: 0 });
    assert_eq!(error.to_string(), "schema field 0 name must not be empty");
}

#[test]
fn checked_schema_reports_duplicate_field_name() {
    let error = FSESchema::try_new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("case_id", FSEFieldType::Text, false),
    ])
    .expect_err("duplicate field name should be rejected");

    assert_eq!(
        error,
        FSESchemaError::DuplicateFieldName {
            name: "case_id".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "schema field name 'case_id' appears more than once"
    );
}

#[test]
#[should_panic(expected = "schema must contain at least one field")]
fn schema_rejects_empty_fields() {
    let _ = FSESchema::new(Vec::new());
}
