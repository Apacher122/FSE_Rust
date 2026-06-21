use crate::data::{FSEField, FSEFieldType, FSESchema, FSEValue};
use crate::query::{
    FSEPredicate, FSEPredicateError, FSEPredicateField, ValidatedFSEPredicateOperator,
};

#[test]
fn typed_predicate_validates_equality_by_field_name() {
    let schema = entity_schema();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("state"),
        FSEValue::Category("active".to_string()),
    );

    let validated = predicate
        .validate(&schema)
        .expect("valid equality predicate should be accepted");

    assert_eq!(validated.field(), 2);
    assert_eq!(validated.name(), "state");
    assert_eq!(validated.field_type(), FSEFieldType::Category);
    assert_eq!(
        validated.operator(),
        &ValidatedFSEPredicateOperator::Equal(FSEValue::Category("active".to_string()))
    );
}

#[test]
fn typed_predicate_validates_range_by_field_index() {
    let schema = entity_schema();
    let predicate = FSEPredicate::range(
        FSEPredicateField::index(1),
        FSEValue::Float(41.0),
        FSEValue::Float(42.0),
    );

    let validated = predicate
        .validate(&schema)
        .expect("valid range predicate should be accepted");

    assert_eq!(validated.field(), 1);
    assert_eq!(validated.name(), "metric");
    assert_eq!(validated.field_type(), FSEFieldType::Float);
    assert_eq!(
        validated.operator(),
        &ValidatedFSEPredicateOperator::Range {
            min: FSEValue::Float(41.0),
            max: FSEValue::Float(42.0),
        }
    );
}

#[test]
fn typed_predicate_reports_unknown_field_name() {
    let schema = entity_schema();
    let predicate = FSEPredicate::equals(FSEPredicateField::name("missing"), FSEValue::Integer(42));

    let error = predicate
        .validate(&schema)
        .expect_err("unknown field name should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::UnknownFieldName {
            name: "missing".to_string(),
        }
    );
    assert_eq!(error.to_string(), "schema field 'missing' was not found");
}

#[test]
fn typed_predicate_reports_field_index_out_of_range() {
    let schema = entity_schema();
    let predicate = FSEPredicate::equals(FSEPredicateField::index(9), FSEValue::Integer(42));

    let error = predicate
        .validate(&schema)
        .expect_err("out-of-range field index should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::FieldIndexOutOfRange {
            field_index: 9,
            field_count: 4,
        }
    );
    assert_eq!(
        error.to_string(),
        "field index 9 is outside schema field count 4"
    );
}

#[test]
fn typed_predicate_reports_null_value() {
    let schema = entity_schema();
    let predicate = FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Null);

    let error = predicate
        .validate(&schema)
        .expect_err("null predicate value should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::NullPredicateValue {
            field: 0,
            name: "entity_id".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate for field 'entity_id' must not be null"
    );
}

#[test]
fn typed_predicate_reports_field_type_mismatch() {
    let schema = entity_schema();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("entity_id"),
        FSEValue::Text("42".to_string()),
    );

    let error = predicate
        .validate(&schema)
        .expect_err("mismatched predicate value type should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::FieldTypeMismatch {
            field: 0,
            name: "entity_id".to_string(),
            expected: FSEFieldType::Integer,
            actual: FSEFieldType::Text,
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate for field 'entity_id' expected Integer but found Text"
    );
}

#[test]
fn typed_predicate_reports_non_finite_float_value() {
    let schema = entity_schema();
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("metric"), FSEValue::Float(f64::NAN));

    let error = predicate
        .validate(&schema)
        .expect_err("non-finite float predicate should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::NonFiniteValue {
            field: 1,
            name: "metric".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate for field 'metric' must use finite values"
    );
}

#[test]
fn typed_predicate_reports_unsupported_range_type() {
    let schema = entity_schema();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("state"),
        FSEValue::Category("archived".to_string()),
        FSEValue::Category("active".to_string()),
    );

    let error = predicate
        .validate(&schema)
        .expect_err("categorical range predicate should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::UnsupportedRangeType {
            field: 2,
            name: "state".to_string(),
            field_type: FSEFieldType::Category,
        }
    );
    assert_eq!(
        error.to_string(),
        "range predicate for field 'state' does not support Category"
    );
}

#[test]
fn typed_predicate_reports_inverted_range() {
    let schema = entity_schema();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("entity_id"),
        FSEValue::Integer(50),
        FSEValue::Integer(42),
    );

    let error = predicate
        .validate(&schema)
        .expect_err("inverted range predicate should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::InvertedRange {
            field: 0,
            name: "entity_id".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "range predicate minimum must not exceed maximum for field 'entity_id'"
    );
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("metric", FSEFieldType::Float, false),
        FSEField::new("state", FSEFieldType::Category, false),
        FSEField::new("observed_at", FSEFieldType::TimestampMillis, false),
    ])
}
