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
    TimestampMillisEncoder, encode_record_batch,
};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryPlan};

#[test]
fn typed_comparison_reports_exact_indexed_execution_metrics() {
    let fixture = typed_fixture();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(10.0),
        FSEValue::Float(11.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");

    let report = compare_typed_query_execution(&fixture.index, &fixture.batch, &plan)
        .expect("typed comparison should execute");

    assert_eq!(report.baseline_matched_records, 2);
    assert_eq!(report.indexed_matched_records, 2);
    assert_eq!(report.indexed_stats.total_records, fixture.batch.len());
    assert_eq!(report.indexed_stats.matched_records, 2);
    assert_eq!(report.indexed_stats.reconstructed_records, 2);
    assert_eq!(report.avoided_record_evaluations, 2);
    assert_eq!(report.record_evaluation_avoidance_ratio, 0.5);
    assert_eq!(report.candidate_ratio, report.indexed_stats.candidate_ratio);
    assert_eq!(
        report.retained_leaf_ratio,
        report.indexed_stats.retained_leaf_ratio
    );
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

    let report = compare_typed_query_execution(&fixture.index, &fixture.batch, &plan)
        .expect("typed comparison should execute");

    assert_eq!(report.baseline_matched_records, 2);
    assert_eq!(report.indexed_matched_records, 2);
    assert_eq!(report.indexed_stats.matched_records, 2);
}

#[test]
fn typed_comparison_uses_requested_repeated_timing_iterations() {
    let fixture = typed_fixture();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("latitude"),
        FSEValue::Float(10.0),
        FSEValue::Float(11.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");
    let timing_config = RepeatedTimingConfig::new(3);

    let report = compare_typed_query_execution_repeated(
        &fixture.index,
        &fixture.batch,
        &plan,
        &timing_config,
    )
    .expect("typed comparison should execute");

    assert_eq!(report.repeated_timing.baseline.iterations, 3);
    assert_eq!(report.repeated_timing.fse.iterations, 3);
}

struct TypedFixture {
    schema: FSESchema,
    mapping: FSESchemaDimensionMapping,
    batch: FSERecordBatch,
    index: crate::build::RowMappedFSEIndex,
}

fn typed_fixture() -> TypedFixture {
    let schema = crime_schema();
    let mapping = crime_mapping(&schema);
    let batch = crime_batch(&schema);
    let encoder = crime_encoder(&schema);
    let encoded = encode_record_batch(&batch, &encoder).expect("valid batch should encode");
    let index = FSEBuilder::new(BuildConfig::new(2, 8))
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("encoded batch should build a row-mapped index");

    TypedFixture {
        schema,
        mapping,
        batch,
        index,
    }
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

fn crime_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
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

fn crime_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(10),
            RowId::new(11),
            RowId::new(12),
            RowId::new(13),
        ],
        vec![
            crime_record(schema, 42, 10.0, "open", 1_735_689_600_000),
            crime_record(schema, 43, 11.0, "open", 1_735_689_650_000),
            crime_record(schema, 44, 1000.0, "closed", 1_735_689_700_000),
            crime_record(schema, 45, 1001.0, "closed", 1_735_689_750_000),
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
