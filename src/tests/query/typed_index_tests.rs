use crate::build::{BuildConfig, BuildInputError, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError,
    FSESchema, FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FSEEncodingError,
    FSERecordBatchEncodingError, FloatEncoder, IntegerEncoder, TimestampMillisEncoder,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, IndexedTypedQueryError, TypedQueryIndex,
    TypedQueryIndexAppendError, TypedQueryIndexBuildError, TypedQueryPlanBuilder,
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
fn typed_query_index_visits_matching_row_ids() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);
    let mut matches = Vec::new();

    let stats = query_index
        .visit_row_ids(&plan, |row_id| {
            matches.push(row_id);
        })
        .expect("typed indexed visitor should execute");

    assert_eq!(matches, vec![RowId::new(100), RowId::new(103)]);
    assert_eq!(stats.total_records, 4);
    assert_eq!(stats.matched_records, 2);
}

#[test]
fn typed_query_index_visits_matching_records() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index = TypedQueryIndex::try_build(batch.clone(), &encoder, &builder())
        .expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);
    let mut matches = Vec::new();

    let stats = query_index
        .visit_rows(&plan, |row_id, record| {
            matches.push((row_id, record.clone()));
        })
        .expect("typed indexed visitor should execute");

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].0, RowId::new(100));
    assert_eq!(
        &matches[0].1,
        batch.record_for_row_id(RowId::new(100)).unwrap()
    );
    assert_eq!(matches[1].0, RowId::new(103));
    assert_eq!(stats.matched_records, 2);
}

#[test]
fn typed_query_index_counts_matches_with_stats() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index = TypedQueryIndex::try_build(batch.clone(), &encoder, &builder())
        .expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);

    let count = query_index
        .count_matches(&plan)
        .expect("typed indexed count should execute");
    let report = query_index
        .count_matches_with_stats(&plan)
        .expect("typed indexed count should execute");

    assert_eq!(count, 2);
    assert_eq!(report.matched_records, 2);
    assert_eq!(report.stats.total_records, batch.len());
    assert_eq!(report.stats.matched_records, 2);
}

#[test]
fn typed_query_index_reports_exact_existence() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index = TypedQueryIndex::try_build(batch.clone(), &encoder, &builder())
        .expect("valid input should build");
    let matching_plan = score_and_class_plan(&schema, &mapping);
    let missing_plan = score_range_plan(&schema, &mapping, 90.0, 100.0);

    assert!(
        query_index
            .has_match(&matching_plan)
            .expect("typed indexed existence should execute")
    );
    assert!(
        !query_index
            .has_match(&missing_plan)
            .expect("typed indexed existence should execute")
    );

    let report = query_index
        .has_match_with_stats(&matching_plan)
        .expect("typed indexed existence should execute");

    assert!(report.has_match);
    assert!(report.inspected_records > 0);
    assert_eq!(report.stats.total_records, batch.len());
    assert_eq!(report.stats.reconstructed_records, report.inspected_records);
    assert_eq!(report.stats.matched_records, 1);
}

#[test]
fn typed_query_index_visitor_reports_missing_records_from_existing_parts() {
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
        .visit_row_ids(&plan, |_row_id| {})
        .expect_err("missing row id should be reported");

    assert_eq!(
        error,
        IndexedTypedQueryError::MissingRecord {
            row_id: RowId::new(103),
        }
    );
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
fn typed_query_index_append_rebuilds_index_and_queries_appended_rows() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid input should build");
    let appended_index = query_index
        .try_append(&appended, &encoder, &builder)
        .expect("valid append should rebuild the index");
    let plan = score_and_class_plan(&schema, &mapping);

    let mut matches = appended_index
        .query_row_ids(&plan)
        .expect("typed indexed query should execute");
    matches.sort();

    assert_eq!(
        matches,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(appended_index.batch().len(), 6);
    assert_eq!(
        appended_index.index().index().root_node().cardinality,
        appended_index.batch().len()
    );
    assert_eq!(
        appended_index
            .batch()
            .record_for_row_id(RowId::new(104))
            .unwrap()
            .value(0),
        Some(&FSEValue::Integer(5))
    );
}

#[test]
fn typed_query_index_append_reports_duplicate_row_ids() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let query_index = TypedQueryIndex::try_build(entity_batch(&schema), &encoder, &builder)
        .expect("valid input should build");
    let appended = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100)],
        vec![entity_record(&schema, 5, 16.0, "alpha", 1_400)],
    );

    assert_eq!(
        query_index.try_append(&appended, &encoder, &builder),
        Err(TypedQueryIndexAppendError::RecordBatch(
            FSERecordBatchError::DuplicateRowId {
                row_id: RowId::new(100)
            }
        ))
    );
}

#[test]
fn typed_query_index_append_reports_schema_mismatch() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let query_index = TypedQueryIndex::try_build(entity_batch(&schema), &encoder, &builder)
        .expect("valid input should build");
    let appended_schema = FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
    ]);
    let appended = FSERecordBatch::new(
        appended_schema.clone(),
        vec![RowId::new(104)],
        vec![FSERecord::new(
            vec![FSEValue::Integer(5), FSEValue::Float(16.0)],
            &appended_schema,
        )],
    );

    assert_eq!(
        query_index.try_append(&appended, &encoder, &builder),
        Err(TypedQueryIndexAppendError::RecordBatch(
            FSERecordBatchError::SchemaMismatch
        ))
    );
}

#[test]
fn typed_query_index_append_reports_empty_append_batch() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let query_index = TypedQueryIndex::try_build(entity_batch(&schema), &encoder, &builder)
        .expect("valid input should build");
    let appended = FSERecordBatch::new(schema, Vec::new(), Vec::new());

    assert_eq!(
        query_index.try_append(&appended, &encoder, &builder),
        Err(TypedQueryIndexAppendError::RecordBatch(
            FSERecordBatchError::EmptyAppendBatch
        ))
    );
}

#[test]
fn typed_query_index_append_propagates_rebuild_errors() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let query_index = TypedQueryIndex::try_build(entity_batch(&schema), &encoder, &builder)
        .expect("valid input should build");
    let appended = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(104)],
        vec![entity_record(&schema, 5, 16.0, "gamma", 1_400)],
    );

    assert_eq!(
        query_index.try_append(&appended, &encoder, &builder),
        Err(TypedQueryIndexAppendError::Rebuild(
            TypedQueryIndexBuildError::Encoding(FSERecordBatchEncodingError::RecordEncoding {
                record: 4,
                row_id: RowId::new(104),
                source: FSEEncodingError::UnsupportedValue {
                    reason: "category 'gamma' is not in dictionary".to_string(),
                },
            })
        ))
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

#[test]
fn typed_query_index_count_reports_missing_records_from_existing_parts() {
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
        .count_matches(&plan)
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

fn score_range_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
    min: f64,
    max: f64,
) -> crate::query::TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(min),
            FSEValue::Float(max),
        ))
        .build()
        .expect("valid predicate should produce a plan")
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

fn appended_entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(104), RowId::new(105)],
        vec![
            entity_record(schema, 5, 16.0, "alpha", 1_400),
            entity_record(schema, 6, 80.0, "beta", 1_500),
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
