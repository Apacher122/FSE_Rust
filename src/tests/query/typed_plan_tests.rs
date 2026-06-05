use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSESchema, FSESchemaDimensionMapping, FSEValue,
};
use crate::encoding::CategoricalDictionaryEncoder;
use crate::math::Scalar;
use crate::query::{
    FSEPredicate, FSEPredicateCompileError, FSEPredicateError, FSEPredicateField, TypedQueryPlan,
    TypedQueryPlanError, ValidatedFSEPredicateOperator,
};

#[test]
fn typed_query_plan_builds_numeric_plan() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(41.0),
        FSEValue::Float(42.0),
    );

    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    assert_eq!(plan.predicate().field(), 1);
    assert_eq!(plan.predicate().name(), "latitude");
    assert_eq!(
        plan.predicate().operator(),
        &ValidatedFSEPredicateOperator::Range {
            min: FSEValue::Float(41.0),
            max: FSEValue::Float(42.0),
        }
    );
    assert_eq!(
        plan.query_region().min,
        vec![Scalar::MIN, 41.0, Scalar::MIN, Scalar::MIN]
    );
    assert_eq!(
        plan.query_region().max,
        vec![Scalar::MAX, 42.0, Scalar::MAX, Scalar::MAX]
    );
}

#[test]
fn typed_query_plan_builds_categorical_equality_plan() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("closed".to_string()),
    );

    let plan = TypedQueryPlan::categorical_equality(&predicate, &schema, &mapping, &encoder)
        .expect("categorical equality predicate should produce a plan");

    assert_eq!(plan.predicate().field(), 2);
    assert_eq!(plan.predicate().name(), "status");
    assert_eq!(
        plan.predicate().operator(),
        &ValidatedFSEPredicateOperator::Equal(FSEValue::Category("closed".to_string()))
    );
    assert_eq!(
        plan.query_region().min,
        vec![Scalar::MIN, Scalar::MIN, 1.0, Scalar::MIN]
    );
    assert_eq!(
        plan.query_region().max,
        vec![Scalar::MAX, Scalar::MAX, 1.0, Scalar::MAX]
    );
}

#[test]
fn typed_query_plan_reports_predicate_validation_error() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let predicate = FSEPredicate::equals(FSEPredicateField::name("missing"), FSEValue::Integer(42));

    let error = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect_err("invalid predicate should not produce a plan");

    assert_eq!(
        error,
        TypedQueryPlanError::Predicate(FSEPredicateError::UnknownFieldName {
            name: "missing".to_string(),
        })
    );
    assert_eq!(error.to_string(), "schema field 'missing' was not found");
}

#[test]
fn typed_query_plan_reports_numeric_compile_error() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("open".to_string()),
    );

    let error = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect_err("categorical predicate should not produce numeric plan");

    assert_eq!(
        error,
        TypedQueryPlanError::Compile(FSEPredicateCompileError::UnsupportedFieldType {
            field: 2,
            name: "status".to_string(),
            field_type: FSEFieldType::Category,
        })
    );
    assert_eq!(
        error.to_string(),
        "predicate field 'status' with type Category cannot be compiled by the numeric predicate compiler"
    );
}

#[test]
fn typed_query_plan_reports_categorical_compile_error() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("pending".to_string()),
    );

    let error = TypedQueryPlan::categorical_equality(&predicate, &schema, &mapping, &encoder)
        .expect_err("unknown category should not produce categorical plan");

    assert_eq!(
        error,
        TypedQueryPlanError::Compile(FSEPredicateCompileError::UnknownCategory {
            field: 2,
            name: "status".to_string(),
            category: "pending".to_string(),
        })
    );
    assert_eq!(
        error.to_string(),
        "category 'pending' for field 'status' is not in dictionary"
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

fn crime_mapping(schema: &FSESchema) -> FSESchemaDimensionMapping {
    FSESchemaDimensionMapping::new(
        schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
            FSEDimensionMapping::new(2, 2),
            FSEDimensionMapping::new(3, 3),
        ],
    )
}

fn status_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["open".to_string(), "closed".to_string()])
}
