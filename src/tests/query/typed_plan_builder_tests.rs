use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSESchema, FSESchemaDimensionMapping, FSEValue,
};
use crate::encoding::CategoricalDictionaryEncoder;
use crate::math::Scalar;
use crate::query::{
    FSEPredicate, FSEPredicateCompileError, FSEPredicateError, FSEPredicateField,
    TypedQueryPlanBuilder, TypedQueryPlanError,
};

#[test]
fn typed_query_plan_builder_builds_numeric_and_categorical_conjunction() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let plan = TypedQueryPlanBuilder::new(&schema, &mapping)
        .with_categorical_encoder(2, class_encoder())
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(10.0),
            FSEValue::Float(20.0),
        ))
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category("alpha".to_string()),
        ))
        .build()
        .expect("valid predicates should produce a typed query plan");

    assert_eq!(plan.predicates().len(), 2);
    assert!(!plan.is_unsatisfiable());
    assert_eq!(
        plan.query_region().min,
        vec![Scalar::MIN, 10.0, 0.0, Scalar::MIN]
    );
    assert_eq!(
        plan.query_region().max,
        vec![Scalar::MAX, 20.0, 0.0, Scalar::MAX]
    );
}

#[test]
fn typed_query_plan_builder_builds_plan_from_pushed_predicates() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let mut builder = TypedQueryPlanBuilder::new(&schema, &mapping);

    builder.push_predicate(FSEPredicate::range(
        FSEPredicateField::name("observed_at"),
        FSEValue::TimestampMillis(1_000),
        FSEValue::TimestampMillis(2_000),
    ));

    let plan = builder
        .build()
        .expect("pushed predicate should produce a typed query plan");

    assert_eq!(plan.predicates().len(), 1);
    assert_eq!(
        plan.query_region().min,
        vec![Scalar::MIN, Scalar::MIN, Scalar::MIN, 1_000.0]
    );
    assert_eq!(
        plan.query_region().max,
        vec![Scalar::MAX, Scalar::MAX, Scalar::MAX, 2_000.0]
    );
}

#[test]
fn typed_query_plan_builder_marks_disjoint_predicates_unsatisfiable() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let plan = TypedQueryPlanBuilder::new(&schema, &mapping)
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(0.0),
            FSEValue::Float(5.0),
        ))
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(10.0),
            FSEValue::Float(20.0),
        ))
        .build()
        .expect("disjoint predicates should still produce a plan");

    assert!(plan.is_unsatisfiable());
    assert_eq!(plan.predicates().len(), 2);
}

#[test]
fn typed_query_plan_builder_reports_missing_categorical_encoder() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let error = TypedQueryPlanBuilder::new(&schema, &mapping)
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category("alpha".to_string()),
        ))
        .build()
        .expect_err("categorical predicates require a registered encoder");

    assert_eq!(
        error,
        TypedQueryPlanError::MissingCategoricalEncoder {
            field: 2,
            name: "class".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "categorical predicate for field 'class' has no registered encoder"
    );
}

#[test]
fn typed_query_plan_builder_reports_predicate_validation_error() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let error = TypedQueryPlanBuilder::new(&schema, &mapping)
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("missing"),
            FSEValue::Integer(42),
        ))
        .build()
        .expect_err("invalid predicate should not produce a plan");

    assert_eq!(
        error,
        TypedQueryPlanError::Predicate(FSEPredicateError::UnknownFieldName {
            name: "missing".to_string(),
        })
    );
}

#[test]
fn typed_query_plan_builder_reports_empty_predicate_list() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let error = TypedQueryPlanBuilder::new(&schema, &mapping)
        .build()
        .expect_err("empty builder should not produce a plan");

    assert_eq!(error, TypedQueryPlanError::EmptyConjunction);
}

#[test]
fn typed_query_plan_builder_reports_unsupported_field_type() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let error = TypedQueryPlanBuilder::new(&schema, &mapping)
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("label"),
            FSEValue::Text("sample".to_string()),
        ))
        .build()
        .expect_err("unsupported field type should not produce a plan");

    assert_eq!(
        error,
        TypedQueryPlanError::Compile(FSEPredicateCompileError::UnsupportedFieldType {
            field: 4,
            name: "label".to_string(),
            field_type: FSEFieldType::Text,
        })
    );
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
        FSEField::new("class", FSEFieldType::Category, false),
        FSEField::new("observed_at", FSEFieldType::TimestampMillis, false),
        FSEField::new("label", FSEFieldType::Text, false),
    ])
}

fn entity_mapping(schema: &FSESchema) -> FSESchemaDimensionMapping {
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

fn class_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["alpha".to_string(), "beta".to_string()])
}
