use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder, encode_record_batch,
};
use crate::math::Scalar;
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedQueryPlan, TypedQueryPlanError,
    evaluate_indexed_typed_query_plan, evaluate_indexed_typed_query_plan_with_stats,
    evaluate_typed_query_plan,
};

#[test]
fn conjunctive_typed_query_plan_intersects_component_regions() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let class_encoder = class_encoder();
    let score_plan = score_range_plan(&schema, &mapping);
    let class_plan = class_equality_plan(&schema, &mapping, &class_encoder, "alpha");

    let plan = TypedQueryPlan::conjunctive(vec![score_plan, class_plan])
        .expect("non-empty conjunctive plan should be accepted");

    assert_eq!(plan.predicates().len(), 2);
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
fn conjunctive_typed_query_plan_reports_empty_component_list() {
    let error = TypedQueryPlan::conjunctive(Vec::new())
        .expect_err("empty conjunctive plan should be rejected");

    assert_eq!(error, TypedQueryPlanError::EmptyConjunction);
    assert_eq!(
        error.to_string(),
        "typed query plan conjunction requires at least one plan"
    );
}

#[test]
fn conjunctive_typed_query_plan_marks_disjoint_component_regions_unsatisfiable() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let low_score = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(0.0),
        FSEValue::Float(5.0),
    );
    let high_score = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(20.0),
    );
    let low_plan = TypedQueryPlan::numeric(&low_score, &schema, &mapping)
        .expect("numeric predicate should produce a plan");
    let high_plan = TypedQueryPlan::numeric(&high_score, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let plan = TypedQueryPlan::conjunctive(vec![low_plan, high_plan])
        .expect("disjoint component regions should produce a plan");

    assert!(plan.is_unsatisfiable());
    assert_eq!(plan.predicates().len(), 2);
}

#[test]
fn conjunctive_typed_scan_requires_every_predicate_to_match() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let class_encoder = class_encoder();
    let score_plan = score_range_plan(&schema, &mapping);
    let class_plan = class_equality_plan(&schema, &mapping, &class_encoder, "alpha");
    let plan = TypedQueryPlan::conjunctive(vec![score_plan, class_plan])
        .expect("non-empty conjunctive plan should be accepted");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert_eq!(matches, vec![RowId::new(100), RowId::new(103)]);
}

#[test]
fn conjunctive_typed_scan_returns_empty_result_for_unsatisfiable_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let low_score = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(0.0),
        FSEValue::Float(5.0),
    );
    let high_score = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(20.0),
    );
    let low_plan = TypedQueryPlan::numeric(&low_score, &schema, &mapping)
        .expect("numeric predicate should produce a plan");
    let high_plan = TypedQueryPlan::numeric(&high_score, &schema, &mapping)
        .expect("numeric predicate should produce a plan");
    let plan = TypedQueryPlan::conjunctive(vec![low_plan, high_plan])
        .expect("disjoint component regions should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert!(matches.is_empty());
}

#[test]
fn conjunctive_indexed_typed_query_returns_matching_row_ids() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");
    let class_encoder = class_encoder();
    let score_plan = score_range_plan(&schema, &mapping);
    let class_plan = class_equality_plan(&schema, &mapping, &class_encoder, "alpha");
    let plan = TypedQueryPlan::conjunctive(vec![score_plan, class_plan])
        .expect("non-empty conjunctive plan should be accepted");

    let mut matches = evaluate_indexed_typed_query_plan(&index, &batch, &plan)
        .expect("indexed typed query should execute");

    matches.sort_by_key(|row_id| row_id.value());
    assert_eq!(matches, vec![RowId::new(100), RowId::new(103)]);
}

#[test]
fn conjunctive_indexed_typed_query_returns_empty_result_for_unsatisfiable_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");
    let low_score = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(0.0),
        FSEValue::Float(5.0),
    );
    let high_score = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(20.0),
    );
    let low_plan = TypedQueryPlan::numeric(&low_score, &schema, &mapping)
        .expect("numeric predicate should produce a plan");
    let high_plan = TypedQueryPlan::numeric(&high_score, &schema, &mapping)
        .expect("numeric predicate should produce a plan");
    let plan = TypedQueryPlan::conjunctive(vec![low_plan, high_plan])
        .expect("disjoint component regions should produce a plan");

    let report = evaluate_indexed_typed_query_plan_with_stats(&index, &batch, &plan)
        .expect("indexed typed query should execute");

    assert!(report.row_ids.is_empty());
    assert_eq!(report.stats.visited_nodes, 0);
    assert_eq!(report.stats.retained_leaves, 0);
    assert_eq!(report.stats.reconstructed_records, 0);
    assert_eq!(report.stats.matched_records, 0);
    assert_eq!(report.stats.total_records, batch.len());
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
        FSEField::new("class", FSEFieldType::Category, false),
        FSEField::new("observed_at", FSEFieldType::TimestampMillis, false),
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

fn entity_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
    ComposedRecordEncoder::new(
        schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(FloatEncoder),
            Box::new(class_encoder()),
            Box::new(TimestampMillisEncoder),
        ],
    )
}

fn entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(100),
            RowId::new(101),
            RowId::new(102),
            RowId::new(103),
        ],
        vec![
            entity_record(schema, 1, 12.5, "alpha", 1_000),
            entity_record(schema, 2, 12.5, "beta", 1_100),
            entity_record(schema, 3, 25.0, "alpha", 1_200),
            entity_record(schema, 4, 18.0, "alpha", 1_300),
        ],
    )
}

fn entity_record(
    schema: &FSESchema,
    entity_id: i64,
    score: f64,
    class: &str,
    observed_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(entity_id),
            FSEValue::Float(score),
            FSEValue::Category(class.to_string()),
            FSEValue::TimestampMillis(observed_at),
        ],
        schema,
    )
}

fn score_range_plan(schema: &FSESchema, mapping: &FSESchemaDimensionMapping) -> TypedQueryPlan {
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(20.0),
    );

    TypedQueryPlan::numeric(&predicate, schema, mapping)
        .expect("numeric predicate should produce a plan")
}

fn class_equality_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
    encoder: &CategoricalDictionaryEncoder,
    class: &str,
) -> TypedQueryPlan {
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("class"),
        FSEValue::Category(class.to_string()),
    );

    TypedQueryPlan::categorical_equality(&predicate, schema, mapping, encoder)
        .expect("categorical predicate should produce a plan")
}

fn class_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["alpha".to_string(), "beta".to_string()])
}
