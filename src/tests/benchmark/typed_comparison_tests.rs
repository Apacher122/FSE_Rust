use crate::benchmark::reports::{
    RepeatedTimingConfig, compare_typed_query_execution, compare_typed_query_execution_repeated,
};
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
    FSEPredicate, FSEPredicateField, TypedQueryExecutionStrategy, TypedQueryIndex, TypedQueryPlan,
    TypedQuerySelectivityBucket,
};

#[test]
fn typed_comparison_reports_exact_indexed_execution_metrics() {
    let fixture = typed_fixture();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(11.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");

    let report = compare_typed_query_execution(&fixture.query_index, &plan)
        .expect("typed comparison should execute");

    assert_eq!(report.baseline_matched_records, 2);
    assert_eq!(report.indexed_matched_records, 2);
    assert_eq!(
        report.indexed_stats.total_records,
        fixture.query_index.batch().len()
    );
    assert_eq!(report.indexed_stats.matched_records, 2);
    assert_eq!(report.indexed_stats.reconstructed_records, 4);
    assert_eq!(report.avoided_record_evaluations, 0);
    assert_eq!(report.record_evaluation_avoidance_ratio, 0.0);
    assert_eq!(report.candidate_ratio, report.indexed_stats.candidate_ratio);
    assert_eq!(
        report.retained_leaf_ratio,
        report.indexed_stats.retained_leaf_ratio
    );
    assert_eq!(
        report.planning_diagnostics.strategy,
        TypedQueryExecutionStrategy::FlatScan
    );
    assert_eq!(
        report.planning_diagnostics.selectivity_bucket,
        TypedQuerySelectivityBucket::Moderate
    );
    assert!(!report.planning_diagnostics.risk_flags.has_any());
    let planner_comparison = report
        .planning_diagnostics
        .cost_comparison_against_flat_scan();

    assert_eq!(
        planner_comparison.selected_strategy,
        TypedQueryExecutionStrategy::FlatScan
    );
    assert!(!planner_comparison.reduces_predicate_evaluations());
    assert!(!planner_comparison.reduces_flat_scan_records());
    assert_eq!(planner_comparison.traversal_node_visit_delta, 0);
}

#[test]
fn typed_comparison_supports_categorical_equality_plan() {
    let fixture = typed_fixture();
    let status_encoder = status_encoder();
    let predicate = FSEPredicate::equals(
        FSEPredicateField::name("status"),
        FSEValue::Category("closed".to_string()),
    );
    let plan = TypedQueryPlan::categorical_equality(
        &predicate,
        &fixture.schema,
        &fixture.mapping,
        &status_encoder,
    )
    .expect("categorical predicate should produce a plan");

    let report = compare_typed_query_execution(&fixture.query_index, &plan)
        .expect("typed comparison should execute");

    assert_eq!(report.baseline_matched_records, 2);
    assert_eq!(report.indexed_matched_records, 2);
    assert_eq!(report.indexed_stats.matched_records, 2);
}

#[test]
fn typed_comparison_uses_requested_repeated_timing_iterations() {
    let fixture = typed_fixture();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(11.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");
    let timing_config = RepeatedTimingConfig::new(3);

    let report =
        compare_typed_query_execution_repeated(&fixture.query_index, &plan, &timing_config)
            .expect("typed comparison should execute");

    assert_eq!(report.repeated_timing.baseline.iterations, 3);
    assert_eq!(report.repeated_timing.fse.iterations, 3);
}

struct TypedFixture {
    schema: FSESchema,
    mapping: FSESchemaDimensionMapping,
    query_index: TypedQueryIndex,
}

fn typed_fixture() -> TypedFixture {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid input should build");

    TypedFixture {
        schema,
        mapping,
        query_index,
    }
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
        FSEField::new("status", FSEFieldType::Category, false),
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
            Box::new(status_encoder()),
            Box::new(TimestampMillisEncoder),
        ],
    )
}

fn entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(10),
            RowId::new(11),
            RowId::new(12),
            RowId::new(13),
        ],
        vec![
            entity_record(schema, 42, 10.0, "open", 1_735_689_600_000),
            entity_record(schema, 43, 11.0, "open", 1_735_689_650_000),
            entity_record(schema, 44, 1000.0, "closed", 1_735_689_700_000),
            entity_record(schema, 45, 1001.0, "closed", 1_735_689_750_000),
        ],
    )
}

fn entity_record(
    schema: &FSESchema,
    entity_id: i64,
    score: f64,
    status: &str,
    observed_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(entity_id),
            FSEValue::Float(score),
            FSEValue::Category(status.to_string()),
            FSEValue::TimestampMillis(observed_at),
        ],
        schema,
    )
}

fn status_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["open".to_string(), "closed".to_string()])
}
