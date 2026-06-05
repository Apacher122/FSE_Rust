use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::CategoricalDictionaryEncoder;
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryPlan, evaluate_typed_query_plan};

#[test]
fn typed_query_plan_evaluation_returns_matching_row_ids_in_batch_order() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let batch = crime_batch(&schema);
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
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
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let batch = crime_batch(&schema);
    let encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("closed".to_string()),
    );
    let plan = TypedQueryPlan::categorical_equality(&predicate, &schema, &mapping, &encoder)
        .expect("categorical equality predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert_eq!(matches, vec![RowId::new(11)]);
}

#[test]
fn typed_query_plan_evaluation_returns_empty_result_when_no_records_match() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let batch = crime_batch(&schema);
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(99));
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert!(matches.is_empty());
}

#[test]
fn typed_query_plan_evaluation_returns_empty_result_for_empty_batch() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let batch = FSERecordBatch::new(schema.clone(), Vec::new(), Vec::new());
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(42));
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert!(matches.is_empty());
}

#[test]
fn typed_query_plan_evaluation_uses_exact_typed_predicate_not_query_region_alone() {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let batch = crime_batch(&schema);
    let predicate = FSEPredicate::equals(FSEPredicateField::name("case_id"), FSEValue::Integer(42));
    let plan = TypedQueryPlan::numeric(&predicate, &schema, &mapping)
        .expect("numeric predicate should produce a plan");

    let matches = evaluate_typed_query_plan(&batch, &plan);

    assert_eq!(matches, vec![RowId::new(10)]);
    assert_eq!(plan.query_region().min[0], 42.0);
    assert_eq!(plan.query_region().max[0], 42.0);
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

fn crime_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(10), RowId::new(11), RowId::new(12)],
        vec![
            crime_record(schema, 42, 41.881, "open", 1_735_689_600_000),
            crime_record(schema, 43, 42.100, "closed", 1_735_689_650_000),
            crime_record(schema, 44, 41.850, "open", 1_735_689_700_000),
        ],
    )
}

fn crime_record(
    schema: &FSESchema,
    case_id: i64,
    latitude: f64,
    status: &str,
    reported_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(case_id),
            FSEValue::Float(latitude),
            FSEValue::Category(status.to_string()),
            FSEValue::TimestampMillis(reported_at),
        ],
        schema,
    )
}

fn status_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["open".to_string(), "closed".to_string()])
}
