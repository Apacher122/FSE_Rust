use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder, encode_record_batch,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, IndexedTypedQueryError, TypedQueryPlan,
    evaluate_indexed_typed_query_plan, evaluate_indexed_typed_query_plan_rows,
    evaluate_indexed_typed_query_plan_with_stats,
};

#[test]
fn indexed_typed_query_plan_returns_matching_row_ids() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(41.8),
        FSEValue::Float(41.9),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_indexed_typed_query_plan(&index, &batch, &plan)
        .expect("indexed typed query should execute");

    assert_eq!(matches, vec![RowId::new(10), RowId::new(12)]);
}

#[test]
fn indexed_typed_query_plan_supports_categorical_equality() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");
    let state_encoder = state_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("state"),
        FSEValue::Category("archived".to_string()),
    );
    let plan = TypedQueryPlan::categorical_equality(&predicate, &schema, &mapping, &state_encoder)
        .expect("categorical predicate should produce a plan");

    let matches = evaluate_indexed_typed_query_plan(&index, &batch, &plan)
        .expect("indexed typed query should execute");

    assert_eq!(matches, vec![RowId::new(11)]);
}

#[test]
fn indexed_typed_query_plan_returns_typed_rows() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(41.8),
        FSEValue::Float(41.9),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let rows = evaluate_indexed_typed_query_plan_rows(&index, &batch, &plan)
        .expect("indexed typed query should execute");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].row_id(), RowId::new(10));
    assert_eq!(
        rows[0].record(),
        batch.record_for_row_id(RowId::new(10)).unwrap()
    );
    assert_eq!(rows[1].row_id(), RowId::new(12));
    assert_eq!(
        rows[1].record(),
        batch.record_for_row_id(RowId::new(12)).unwrap()
    );
}

#[test]
fn indexed_typed_query_plan_reports_geometric_execution_stats() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(41.8),
        FSEValue::Float(41.9),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let report = evaluate_indexed_typed_query_plan_with_stats(&index, &batch, &plan)
        .expect("indexed typed query should execute");

    assert_eq!(report.row_ids, vec![RowId::new(10), RowId::new(12)]);
    assert_eq!(report.stats.total_records, batch.len());
    assert_eq!(report.stats.matched_records, 2);
}

#[test]
fn indexed_typed_query_plan_reports_missing_record() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let indexed_batch = entity_batch(&schema);
    let query_batch = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10), RowId::new(11)],
        vec![
            entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000),
            entity_record(&schema, 43, 42.100, "archived", 1_735_689_650_000),
        ],
    );
    let encoder = entity_encoder(&schema);
    let encoded = encode_record_batch(&indexed_batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(41.8),
        FSEValue::Float(41.9),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let error = evaluate_indexed_typed_query_plan(&index, &query_batch, &plan)
        .expect_err("missing row id should be reported");

    assert_eq!(
        error,
        IndexedTypedQueryError::MissingRecord {
            row_id: RowId::new(12)
        }
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
            Box::new(state_encoder()),
            Box::new(TimestampMillisEncoder),
        ],
    )
}

fn entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10), RowId::new(11), RowId::new(12)],
        vec![
            entity_record(schema, 42, 41.881, "active", 1_735_689_600_000),
            entity_record(schema, 43, 42.100, "archived", 1_735_689_650_000),
            entity_record(schema, 44, 41.850, "active", 1_735_689_700_000),
        ],
    )
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

fn state_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["active".to_string(), "archived".to_string()])
}
