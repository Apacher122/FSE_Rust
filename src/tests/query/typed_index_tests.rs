use crate::build::{BuildConfig, BuildInputError, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FSEEncodingError,
    FSERecordBatchEncodingError, FloatEncoder, IntegerEncoder, TimestampMillisEncoder,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, IndexedTypedQueryError, TypedQueryIndex,
    TypedQueryIndexBuildError, TypedQueryPlanBuilder,
};

#[test]
fn typed_query_index_builds_from_record_batch_and_queries_row_ids() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);

    let matches = query_index
        .query_row_ids(&plan)
        .expect("typed indexed query should execute");

    assert_eq!(matches, vec![RowId::new(100), RowId::new(103)]);
}

#[test]
fn typed_query_index_returns_rows_with_stats() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index = TypedQueryIndex::try_build(batch.clone(), &encoder, &builder())
        .expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);

    let report = query_index
        .query_rows_with_stats(&plan)
        .expect("typed indexed query should execute");

    assert_eq!(report.rows.len(), 2);
    assert_eq!(report.rows[0].row_id(), RowId::new(100));
    assert_eq!(
        report.rows[0].record(),
        batch.record_for_row_id(RowId::new(100)).unwrap()
    );
    assert_eq!(report.rows[1].row_id(), RowId::new(103));
    assert_eq!(report.stats.total_records, batch.len());
    assert_eq!(report.stats.matched_records, 2);
}

#[test]
fn typed_query_index_returns_index_and_batch_references() {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index = TypedQueryIndex::try_build(batch.clone(), &encoder, &builder())
        .expect("valid input should build");

    assert_eq!(query_index.batch(), &batch);
    assert_eq!(
        query_index.index().index().root_node().cardinality,
        batch.len()
    );
}

#[test]
fn typed_query_index_propagates_encoding_errors() {
    let schema = entity_schema();
    let batch = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100)],
        vec![entity_record(&schema, 1, 12.5, "gamma", 1_000)],
    );
    let encoder = entity_encoder(&schema);

    let error = TypedQueryIndex::try_build(batch, &encoder, &builder())
        .expect_err("unknown category should fail during encoding");

    assert_eq!(
        error,
        TypedQueryIndexBuildError::Encoding(FSERecordBatchEncodingError::RecordEncoding {
            record: 0,
            row_id: RowId::new(100),
            source: FSEEncodingError::UnsupportedValue {
                reason: "category 'gamma' is not in dictionary".to_string(),
            },
        })
    );
}

#[test]
fn typed_query_index_propagates_build_errors() {
    let schema = entity_schema();
    let batch = FSERecordBatch::new(schema.clone(), Vec::new(), Vec::new());
    let encoder = entity_encoder(&schema);

    let error = TypedQueryIndex::try_build(batch, &encoder, &builder())
        .expect_err("empty encoded input should fail during build");

    assert_eq!(
        error,
        TypedQueryIndexBuildError::Build(BuildInputError::EmptyPointSet)
    );
}

#[test]
fn typed_query_index_reports_missing_records_from_existing_parts() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let indexed_batch = entity_batch(&schema);
    let query_batch = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100), RowId::new(101), RowId::new(102)],
        vec![
            entity_record(&schema, 1, 12.5, "alpha", 1_000),
            entity_record(&schema, 2, 12.5, "beta", 1_100),
            entity_record(&schema, 3, 25.0, "alpha", 1_200),
        ],
    );
    let encoder = entity_encoder(&schema);
    let query_index = TypedQueryIndex::try_build(indexed_batch, &encoder, &builder())
        .expect("valid input should build");
    let query_index = TypedQueryIndex::from_parts(query_batch, query_index.index().clone());
    let plan = score_and_class_plan(&schema, &mapping);

    let error = query_index
        .query_row_ids(&plan)
        .expect_err("missing row id should be reported");

    assert_eq!(
        error,
        IndexedTypedQueryError::MissingRecord {
            row_id: RowId::new(103),
        }
    );
}

fn score_and_class_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
) -> crate::query::TypedQueryPlan {
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

fn class_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["alpha".to_string(), "beta".to_string()])
}

fn builder() -> FSEBuilder {
    FSEBuilder::new(BuildConfig::new(2, 8))
}
