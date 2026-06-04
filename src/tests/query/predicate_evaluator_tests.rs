use crate::data::{FSEField, FSEFieldType, FSERecord, FSESchema, FSEValue};
use crate::query::{FSEPredicate, FSEPredicateField, evaluate_typed_predicate};

#[test]
fn typed_predicate_evaluator_matches_integer_equality() {
    let schema = crime_schema();
    let record = crime_record(&schema, 42, 41.881, "open", 1_735_689_600_000);
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(42))
        .validate(&schema)
        .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_rejects_non_matching_equality() {
    let schema = crime_schema();
    let record = crime_record(&schema, 42, 41.881, "open", 1_735_689_600_000);
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(43))
        .validate(&schema)
        .expect("valid predicate should validate");

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_matches_categorical_equality() {
    let schema = crime_schema();
    let record = crime_record(&schema, 42, 41.881, "open", 1_735_689_600_000);
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("open".to_string()),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_matches_inclusive_float_range() {
    let schema = crime_schema();
    let record = crime_record(&schema, 42, 41.881, "open", 1_735_689_600_000);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(41.881),
        FSEValue::Float(41.881),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_rejects_value_outside_range() {
    let schema = crime_schema();
    let record = crime_record(&schema, 42, 41.881, "open", 1_735_689_600_000);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(42.0),
        FSEValue::Float(43.0),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_matches_timestamp_range() {
    let schema = crime_schema();
    let record = crime_record(&schema, 42, 41.881, "open", 1_735_689_600_000);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("reported_at"),
        FSEValue::TimestampMillis(1_735_689_500_000),
        FSEValue::TimestampMillis(1_735_689_700_000),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_returns_false_for_missing_field() {
    let schema = crime_schema();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("reported_at"),
        FSEValue::TimestampMillis(1_735_689_600_000),
    )
    .validate(&schema)
    .expect("valid predicate should validate");
    let shorter_schema =
        FSESchema::new(vec![FSEField::new("case_id", FSEFieldType::Integer, false)]);
    let record = FSERecord::new(vec![FSEValue::Integer(42)], &shorter_schema);

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_returns_false_for_null_record_value() {
    let schema = nullable_case_schema();
    let record = FSERecord::new(vec![FSEValue::Null], &schema);
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(42))
        .validate(&schema)
        .expect("valid predicate should validate");

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_returns_false_for_record_type_mismatch() {
    let schema = crime_schema();
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(42))
        .validate(&schema)
        .expect("valid predicate should validate");
    let text_schema = FSESchema::new(vec![FSEField::new("case_id", FSEFieldType::Text, false)]);
    let record = FSERecord::new(vec![FSEValue::Text("42".to_string())], &text_schema);

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("status", FSEFieldType::Category, false),
        FSEField::new("reported_at", FSEFieldType::TimestampMillis, false),
    ])
}

fn nullable_case_schema() -> FSESchema {
    FSESchema::new(vec![FSEField::new("case_id", FSEFieldType::Integer, true)])
}

fn crime_record(
    schema: &FSESchema,
    case_id: i64,
    latitude: f64,
    status: &str,
    reported_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(case_id),
            FSEValue::Float(latitude),
            FSEValue::Category(status.to_string()),
            FSEValue::TimestampMillis(reported_at),
        ],
        schema,
    )
}
