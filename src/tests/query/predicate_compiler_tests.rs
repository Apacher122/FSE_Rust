use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSESchema, FSESchemaDimensionMapping, FSEValue,
};
use crate::encoding::CategoricalDictionaryEncoder;
use crate::math::Scalar;
use crate::query::{
    FSEPredicate, FSEPredicateCompileError, FSEPredicateField,
    compile_categorical_equality_predicate_to_query_region,
    compile_numeric_predicate_to_query_region,
};

#[test]
fn numeric_predicate_compiler_compiles_integer_equality() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(42))
        .validate(&schema)
        .expect("valid predicate should validate");

    let region = compile_numeric_predicate_to_query_region(&predicate, &mapping)
        .expect("numeric equality predicate should compile");

    assert_eq!(
        region.min,
        vec![42.0, Scalar::MIN, Scalar::MIN, Scalar::MIN]
    );
    assert_eq!(
        region.max,
        vec![42.0, Scalar::MAX, Scalar::MAX, Scalar::MAX]
    );
}

#[test]
fn numeric_predicate_compiler_compiles_float_range() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(41.0),
        FSEValue::Float(42.0),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let region = compile_numeric_predicate_to_query_region(&predicate, &mapping)
        .expect("numeric range predicate should compile");

    assert_eq!(
        region.min,
        vec![Scalar::MIN, 41.0, Scalar::MIN, Scalar::MIN]
    );
    assert_eq!(
        region.max,
        vec![Scalar::MAX, 42.0, Scalar::MAX, Scalar::MAX]
    );
}

#[test]
fn numeric_predicate_compiler_compiles_timestamp_range() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("reported_at"),
        FSEValue::TimestampMillis(1_735_689_600_000),
        FSEValue::TimestampMillis(1_735_689_700_000),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let region = compile_numeric_predicate_to_query_region(&predicate, &mapping)
        .expect("timestamp range predicate should compile");

    assert_eq!(region.min[3], 1_735_689_600_000_i64 as Scalar);
    assert_eq!(region.max[3], 1_735_689_700_000_i64 as Scalar);
}

#[test]
fn numeric_predicate_compiler_reports_unmapped_field() {
    let schema = crime_schema();
    let mapping = FSESchemaDimensionMapping::new(
        &schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
        ],
    );
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("reported_at"),
        FSEValue::TimestampMillis(1_735_689_600_000),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let error = compile_numeric_predicate_to_query_region(&predicate, &mapping)
        .expect_err("unmapped predicate field should be rejected");

    assert_eq!(
        error,
        FSEPredicateCompileError::FieldNotMapped {
            field: 3,
            name: "reported_at".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate field 'reported_at' has no coordinate mapping"
    );
}

#[test]
fn numeric_predicate_compiler_reports_multiple_field_mappings() {
    let schema = crime_schema();
    let mapping = FSESchemaDimensionMapping::new(
        &schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
            FSEDimensionMapping::new(1, 2),
        ],
    );
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(41.0),
        FSEValue::Float(42.0),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let error = compile_numeric_predicate_to_query_region(&predicate, &mapping)
        .expect_err("multi-dimensional predicate field should be rejected");

    assert_eq!(
        error,
        FSEPredicateCompileError::FieldMappedToMultipleDimensions {
            field: 1,
            name: "latitude".to_string(),
            dimensions: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate field 'latitude' maps to 2 coordinate dimensions"
    );
}

#[test]
fn numeric_predicate_compiler_reports_unsupported_field_type() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("open".to_string()),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let error = compile_numeric_predicate_to_query_region(&predicate, &mapping)
        .expect_err("categorical predicate should not compile through numeric compiler");

    assert_eq!(
        error,
        FSEPredicateCompileError::UnsupportedFieldType {
            field: 2,
            name: "status".to_string(),
            field_type: FSEFieldType::Category,
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate field 'status' with type Category cannot be compiled by the numeric predicate compiler"
    );
}

#[test]
fn categorical_predicate_compiler_compiles_equality() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("closed".to_string()),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let region =
        compile_categorical_equality_predicate_to_query_region(&predicate, &mapping, &encoder)
            .expect("categorical equality predicate should compile");

    assert_eq!(region.min, vec![Scalar::MIN, Scalar::MIN, 1.0, Scalar::MIN]);
    assert_eq!(region.max, vec![Scalar::MAX, Scalar::MAX, 1.0, Scalar::MAX]);
}

#[test]
fn categorical_predicate_compiler_reports_unsupported_field_type() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(42))
        .validate(&schema)
        .expect("valid predicate should validate");

    let error =
        compile_categorical_equality_predicate_to_query_region(&predicate, &mapping, &encoder)
            .expect_err("numeric predicate should not compile through categorical compiler");

    assert_eq!(
        error,
        FSEPredicateCompileError::UnsupportedCategoricalFieldType {
            field: 0,
            name: "case_id".to_string(),
            field_type: FSEFieldType::Integer,
        }
    );
    assert_eq!(
        error.to_string(),
        "predicate field 'case_id' with type Integer cannot be compiled by the categorical predicate compiler"
    );
}

#[test]
fn categorical_predicate_compiler_reports_unknown_category() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("pending".to_string()),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let error =
        compile_categorical_equality_predicate_to_query_region(&predicate, &mapping, &encoder)
            .expect_err("unknown category should not compile");

    assert_eq!(
        error,
        FSEPredicateCompileError::UnknownCategory {
            field: 2,
            name: "status".to_string(),
            category: "pending".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "category 'pending' for field 'status' is not in dictionary"
    );
}

#[test]
fn categorical_predicate_compiler_reports_unmapped_field() {
    let schema = crime_schema();
    let mapping = FSESchemaDimensionMapping::new(
        &schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
            FSEDimensionMapping::new(3, 2),
        ],
    );
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("open".to_string()),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let error =
        compile_categorical_equality_predicate_to_query_region(&predicate, &mapping, &encoder)
            .expect_err("unmapped categorical field should not compile");

    assert_eq!(
        error,
        FSEPredicateCompileError::FieldNotMapped {
            field: 2,
            name: "status".to_string(),
        }
    );
}

#[test]
fn categorical_predicate_compiler_reports_multiple_field_mappings() {
    let schema = crime_schema();
    let mapping = FSESchemaDimensionMapping::new(
        &schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
            FSEDimensionMapping::new(2, 2),
            FSEDimensionMapping::new(2, 3),
        ],
    );
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("open".to_string()),
    )
    .validate(&schema)
    .expect("valid predicate should validate");

    let error =
        compile_categorical_equality_predicate_to_query_region(&predicate, &mapping, &encoder)
            .expect_err("multi-dimensional categorical field should not compile");

    assert_eq!(
        error,
        FSEPredicateCompileError::FieldMappedToMultipleDimensions {
            field: 2,
            name: "status".to_string(),
            dimensions: 2,
        }
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
