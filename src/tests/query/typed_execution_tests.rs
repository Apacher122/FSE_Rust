use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::CategoricalDictionaryEncoder;
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedQueryPlan, TypedQueryResultRow,
    evaluate_typed_query_plan, evaluate_typed_query_plan_rows,
};

#[test]
fn typed_query_plan_evaluation_returns_matching_row_ids_in_batch_order() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(41.8),
        FSEValue::Float(41.9),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert_eq!(matches, vec![RowId::new(10), RowId::new(12)]);
}

#[test]
fn typed_query_plan_evaluation_supports_categorical_equality_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = state_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("state"),
        FSEValue::Category("archived".to_string()),
    );
    let plan = TypedQueryPlan::categorical_equality(&predicate, &schema, &mapping, &encoder)
        .expect("categorical equality predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert_eq!(matches, vec![RowId::new(11)]);
}

#[test]
fn typed_query_plan_evaluation_returns_empty_result_when_no_records_match() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(99));
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert!(matches.is_empty());
}

#[test]
fn typed_query_plan_evaluation_returns_empty_result_for_empty_batch() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = FSERecordBatch::new(schema.clone(), Vec::new(), Vec::new());
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(42));
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert!(matches.is_empty());
}

#[test]
fn typed_query_plan_evaluation_uses_exact_typed_predicate_not_query_region_alone() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(42));
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert_eq!(matches, vec![RowId::new(10)]);
    assert_eq!(plan.query_region().min[0], 42.0);
    assert_eq!(plan.query_region().max[0], 42.0);
}

#[test]
fn typed_query_plan_row_evaluation_returns_matching_rows_in_batch_order() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("metric"),
        FSEValue::Float(41.8),
        FSEValue::Float(41.9),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let rows = evaluate_typed_query_plan_rows(&batch, &plan);

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
fn typed_query_plan_row_evaluation_returns_empty_rows_when_no_records_match() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let predicate =
        FSEPredicate::equals(FSEPredicateField::name("entity_id"), FSEValue::Integer(99));
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let rows = evaluate_typed_query_plan_rows(&batch, &plan);

    assert!(rows.is_empty());
}

#[test]
fn typed_query_result_row_exposes_row_id_and_record() {
    let schema = entity_schema();
    let record = entity_record(&schema, 42, 41.881, "active", 1_735_689_600_000);
    let row = TypedQueryResultRow::new(RowId::new(10), record.clone());

    assert_eq!(row.row_id(), RowId::new(10));
    assert_eq!(row.record(), &record);
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
