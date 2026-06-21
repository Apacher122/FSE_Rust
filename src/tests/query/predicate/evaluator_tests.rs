use crate::data::{FSEField, FSEFieldType, FSERecord, FSESchema, FSEValue};
use crate::query::{FSEPredicate, FSEPredicateField, evaluate_typed_predicate};

#[test]
fn typed_predicate_evaluator_matches_integer_equality() {
    let schema = entity_schema();
    let record = entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000);
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(42))
            .validate(&schema)
            .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_rejects_non_matching_equality() {
    let schema = entity_schema();
    let record = entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000);
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(43))
            .validate(&schema)
            .expect("valid predicate should validate");

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_matches_categorical_equality() {
    let schema = entity_schema();
    let record = entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000);
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("state"),
        FSEValue::Category("active".to_string()),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_matches_inclusive_float_range() {
    let schema = entity_schema();
    let record = entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(41.881),
        FSEValue::Float(41.881),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_rejects_value_outside_range() {
    let schema = entity_schema();
    let record = entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(42.0),
        FSEValue::Float(43.0),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_matches_timestamp_range() {
    let schema = entity_schema();
    let record = entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("observed_at"),
        FSEValue::TimestampMillis(1_735_689_500_000),
        FSEValue::TimestampMillis(1_735_689_700_000),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    assert!(evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_returns_false_for_missing_field() {
    let schema = entity_schema();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("observed_at"),
        FSEValue::TimestampMillis(1_735_689_600_000),
    )
    .validate(&schema)
    .expect("valid predicate should validate");
    let shorter_schema = FSESchema::new(vec![FSEField::new(
        "entity_id",
        FSEFieldType::Integer,
        false,
    )]);
    let record = FSERecord::new(vec![FSEValue::Integer(42)], &shorter_schema);

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_returns_false_for_null_record_value() {
    let schema = nullable_case_schema();
    let record = FSERecord::new(vec![FSEValue::Null], &schema);
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(42))
            .validate(&schema)
            .expect("valid predicate should validate");

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

#[test]
fn typed_predicate_evaluator_returns_false_for_record_type_mismatch() {
    let schema = entity_schema();
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(42))
            .validate(&schema)
            .expect("valid predicate should validate");
    let text_schema = FSESchema::new(vec![FSEField::new("entity_id", FSEFieldType::Text, false)]);
    let record = FSERecord::new(vec![FSEValue::Text("42".to_string())], &text_schema);

    assert!(!evaluate_typed_predicate(&record, &predicate));
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("metric", FSEFieldType::Float, false),
        FSEField::new("state", FSEFieldType::Category, false),
        FSEField::new("observed_at", FSEFieldType::TimestampMillis, false),
    ])
}

fn nullable_case_schema() -> FSESchema {
    FSESchema::new(vec![FSEField::new(
        "entity_id",
        FSEFieldType::Integer,
        true,
    )])
}

fn entity_record(
    schema: &FSESchema,
    entity_id: i64,
    metric: f64,
    state: &str,
    observed_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(entity_id),
            FSEValue::Float(metric),
            FSEValue::Category(state.to_string()),
            FSEValue::TimestampMillis(observed_at),
        ],
        schema,
    )
}
