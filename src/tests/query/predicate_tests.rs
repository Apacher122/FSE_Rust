use crate::data::{FSEField, FSEFieldType, FSESchema, FSEValue};
use crate::query::{
    FSEPredicate, FSEPredicateError, FSEPredicateField, ValidatedFSEPredicateOperator,
};

#[test]
fn typed_predicate_validates_equality_by_field_name() {
    let schema = crime_schema();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("open".to_string()),
    );

    let validated = predicate
        .validate(&schema)
        .expect("valid equality predicate should be accepted");

    assert_eq!(validated.field(), 2);
    assert_eq!(validated.name(), "status");
    assert_eq!(validated.field_type(), FSEFieldType::Category);
    assert_eq!(
        validated.operator(),
        &ValidatedFSEPredicateOperator::Equal(FSEValue::Category("open".to_string()))
    );
}

#[test]
fn typed_predicate_validates_range_by_field_index() {
    let schema = crime_schema();
    let predicate = FSEPredicate::range(
        FSEPredicateField::index(1),
        FSEValue::Float(41.0),
        FSEValue::Float(42.0),
    );

    let validated = predicate
        .validate(&schema)
        .expect("valid range predicate should be accepted");

    assert_eq!(validated.field(), 1);
    assert_eq!(validated.name(), "latitude");
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
    let schema = crime_schema();
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
    let schema = crime_schema();
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
    let schema = crime_schema();
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Null);

    let error = predicate
        .validate(&schema)
        .expect_err("null predicate value should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::NullPredicateValue {
            field: 0,
            name: "case_id".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate for field 'case_id' must not be null"
    );
}

#[test]
fn typed_predicate_reports_field_type_mismatch() {
    let schema = crime_schema();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("case_id"),
        FSEValue::Text("42".to_string()),
    );

    let error = predicate
        .validate(&schema)
        .expect_err("mismatched predicate value type should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::FieldTypeMismatch {
            field: 0,
            name: "case_id".to_string(),
            expected: FSEFieldType::Integer,
            actual: FSEFieldType::Text,
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate for field 'case_id' expected Integer but found Text"
    );
}

#[test]
fn typed_predicate_reports_non_finite_float_value() {
    let schema = crime_schema();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(f64::NAN),
    );

    let error = predicate
        .validate(&schema)
        .expect_err("non-finite float predicate should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::NonFiniteValue {
            field: 1,
            name: "latitude".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate for field 'latitude' must use finite values"
    );
}

#[test]
fn typed_predicate_reports_unsupported_range_type() {
    let schema = crime_schema();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("status"),
        FSEValue::Category("closed".to_string()),
        FSEValue::Category("open".to_string()),
    );

    let error = predicate
        .validate(&schema)
        .expect_err("categorical range predicate should be rejected");

    assert_eq!(
        error,
        FSEPredicateError::UnsupportedRangeType {
            field: 2,
            name: "status".to_string(),
            field_type: FSEFieldType::Category,
        }
    );
    assert_eq!(
        error.to_string(),
        "range predicate for field 'status' does not support Category"
    );
}

#[test]
fn typed_predicate_reports_inverted_range() {
    let schema = crime_schema();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("case_id"),
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
            name: "case_id".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "range predicate minimum must not exceed maximum for field 'case_id'"
    );
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("status", FSEFieldType::Category, false),
        FSEField::new("reported_at", FSEFieldType::TimestampMillis, false),
    ])
}
