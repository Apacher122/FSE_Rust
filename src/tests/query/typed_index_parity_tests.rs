use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryPlan, TypedQueryPlanBuilder,
    TypedQueryResultRow, evaluate_typed_query_plan, evaluate_typed_query_plan_rows,
};

#[test]
fn typed_index_matches_flat_scan_for_numeric_range() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let query_index = typed_query_index(&batch);
    let plan = score_range_plan(&schema, &mapping, 10.0, 20.0);

    assert_row_id_parity(&batch, &query_index, &plan);
}

#[test]
fn typed_index_matches_flat_scan_for_categorical_equality() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let query_index = typed_query_index(&batch);
    let plan = class_equality_plan(&schema, &mapping, "alpha");

    assert_row_id_parity(&batch, &query_index, &plan);
}

#[test]
fn typed_index_matches_flat_scan_for_conjunctive_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let query_index = typed_query_index(&batch);
    let plan = score_and_class_plan(&schema, &mapping);

    assert_row_id_parity(&batch, &query_index, &plan);
}

#[test]
fn typed_index_matches_flat_scan_for_unsatisfiable_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let query_index = typed_query_index(&batch);
    let low_score = score_range_plan(&schema, &mapping, 0.0, 5.0);
    let high_score = score_range_plan(&schema, &mapping, 10.0, 20.0);
    let plan = TypedQueryPlan::conjunctive(vec![low_score, high_score])
        .expect("valid components should produce an unsatisfiable plan");

    assert!(plan.is_unsatisfiable());
    assert_row_id_parity(&batch, &query_index, &plan);
}

#[test]
fn typed_index_matches_flat_scan_for_empty_result() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let query_index = typed_query_index(&batch);
    let plan = class_equality_plan(&schema, &mapping, "missing");

    assert_row_id_parity(&batch, &query_index, &plan);
}

#[test]
fn typed_index_row_results_match_flat_scan_rows() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let query_index = typed_query_index(&batch);
    let plan = score_and_class_plan(&schema, &mapping);
    let mut expected = evaluate_typed_query_plan_rows(&batch, &plan);
    let mut actual = query_index
        .query_rows(&plan)
        .expect("typed indexed query should execute");

    sort_rows(&mut expected);
    sort_rows(&mut actual);

    assert_eq!(actual, expected);
}

fn assert_row_id_parity(
    batch: &FSERecordBatch,
    query_index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
) {
    let mut expected = evaluate_typed_query_plan(batch, plan);
    let mut actual = query_index
        .query_row_ids(plan)
        .expect("typed indexed query should execute");

    expected.sort_by_key(|row_id| row_id.value());
    actual.sort_by_key(|row_id| row_id.value());

    assert_eq!(actual, expected);
}

fn sort_rows(rows: &mut [TypedQueryResultRow]) {
    rows.sort_by_key(|row| row.row_id().value());
}

fn score_range_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
    min: f64,
    max: f64,
) -> TypedQueryPlan {
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(min),
        FSEValue::Float(max),
    );

    TypedQueryPlan::numeric(&predicate, schema, mapping)
        .expect("numeric predicate should produce a plan")
}

fn class_equality_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
    class: &str,
) -> TypedQueryPlan {
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("class"),
        FSEValue::Category(class.to_string()),
    );

    TypedQueryPlan::categorical_equality(&predicate, schema, mapping, &class_encoder())
        .expect("categorical predicate should produce a plan")
}

fn score_and_class_plan(schema: &FSESchema, mapping: &FSESchemaDimensionMapping) -> TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
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
        .expect("valid predicates should produce a plan")
}

fn typed_query_index(batch: &FSERecordBatch) -> TypedQueryIndex {
    TypedQueryIndex::try_build(batch.clone(), &entity_encoder(batch.schema()), &builder())
        .expect("valid input should build")
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
            RowId::new(104),
        ],
        vec![
            entity_record(schema, 1, 12.5, "alpha", 1_000),
            entity_record(schema, 2, 12.5, "beta", 1_100),
            entity_record(schema, 3, 25.0, "alpha", 1_200),
            entity_record(schema, 4, 18.0, "alpha", 1_300),
            entity_record(schema, 5, 18.0, "beta", 1_400),
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

fn class_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec![
        "alpha".to_string(),
        "beta".to_string(),
        "missing".to_string(),
    ])
}

fn builder() -> FSEBuilder {
    FSEBuilder::new(BuildConfig::new(2, 8))
}
