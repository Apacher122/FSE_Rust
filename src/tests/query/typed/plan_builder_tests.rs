use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSESchema, FSESchemaDimensionMapping, FSEValue,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, FSEFieldEncoderMetadata, FSERecordEncoderMetadata,
    FSERecordEncoderMetadataError,
};
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
fn typed_query_plan_builder_uses_record_encoder_metadata_for_categorical_predicates() {
    let schema = encoded_entity_schema();
    let mapping = encoded_entity_mapping(&schema);
    let metadata = encoder_metadata();
    let plan = TypedQueryPlanBuilder::new(&schema, &mapping)
        .try_with_record_encoder_metadata(&metadata)
        .expect("valid encoder metadata should register categorical encoders")
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category("alpha".to_string()),
        ))
        .build()
        .expect("valid categorical predicate should produce a typed query plan");

    assert_eq!(
        plan.query_region().min,
        vec![Scalar::MIN, Scalar::MIN, 0.0, Scalar::MIN]
    );
    assert_eq!(
        plan.query_region().max,
        vec![Scalar::MAX, Scalar::MAX, 0.0, Scalar::MAX]
    );
}

#[test]
fn typed_query_plan_builder_preserves_record_encoder_metadata_category_order() {
    let schema = encoded_entity_schema();
    let mapping = encoded_entity_mapping(&schema);
    let metadata = reverse_category_encoder_metadata();
    let plan = TypedQueryPlanBuilder::new(&schema, &mapping)
        .try_with_record_encoder_metadata(&metadata)
        .expect("valid encoder metadata should register categorical encoders")
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category("alpha".to_string()),
        ))
        .build()
        .expect("valid categorical predicate should produce a typed query plan");

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
fn typed_query_plan_builder_reports_record_encoder_metadata_mismatch() {
    let schema = encoded_entity_schema();
    let mapping = encoded_entity_mapping(&schema);
    let metadata = FSERecordEncoderMetadata::new(vec![
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::CategoryDictionary {
            categories: vec!["alpha".to_string(), "beta".to_string()],
        },
        FSEFieldEncoderMetadata::TimestampMillis,
    ]);

    let error = TypedQueryPlanBuilder::new(&schema, &mapping)
        .try_with_record_encoder_metadata(&metadata)
        .expect_err("mismatched metadata should not be accepted");

    assert_eq!(
        error,
        TypedQueryPlanError::EncoderMetadata(FSERecordEncoderMetadataError::FieldTypeMismatch {
            field: 1,
            name: "score".to_string(),
            expected: FSEFieldType::Float,
            actual: FSEFieldType::Integer,
        })
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

fn encoded_entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
        FSEField::new("class", FSEFieldType::Category, false),
        FSEField::new("observed_at", FSEFieldType::TimestampMillis, false),
    ])
}

fn encoded_entity_mapping(schema: &FSESchema) -> FSESchemaDimensionMapping {
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

fn encoder_metadata() -> FSERecordEncoderMetadata {
    FSERecordEncoderMetadata::new(vec![
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::Float,
        FSEFieldEncoderMetadata::CategoryDictionary {
            categories: vec!["alpha".to_string(), "beta".to_string()],
        },
        FSEFieldEncoderMetadata::TimestampMillis,
    ])
}

fn reverse_category_encoder_metadata() -> FSERecordEncoderMetadata {
    FSERecordEncoderMetadata::new(vec![
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::Float,
        FSEFieldEncoderMetadata::CategoryDictionary {
            categories: vec!["beta".to_string(), "alpha".to_string()],
        },
        FSEFieldEncoderMetadata::TimestampMillis,
    ])
}
