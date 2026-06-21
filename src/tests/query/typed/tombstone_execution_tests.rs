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
    FSEPredicate, FSEPredicateField, IndexedTypedQueryError, TypedQueryIndex,
    TypedQueryPlanBuilder, TypedRowTombstoneSet,
};

#[test]
fn typed_row_tombstone_set_deduplicates_and_sorts_row_ids() {
    let tombstones =
        TypedRowTombstoneSet::from_row_ids(vec![RowId::new(103), RowId::new(100), RowId::new(103)]);

    assert_eq!(tombstones.row_ids(), &[RowId::new(100), RowId::new(103)]);
    assert_eq!(tombstones.len(), 2);
    assert!(tombstones.contains(RowId::new(100)));
    assert!(!tombstones.contains(RowId::new(101)));
}

#[test]
fn typed_query_index_excludes_tombstoned_row_ids_from_result_contracts() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index(&schema);
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids(vec![RowId::new(100)]);

    assert_eq!(
        query_index.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );
    assert_eq!(
        query_index
            .query_row_ids_excluding_tombstones(&plan, &tombstones)
            .unwrap(),
        vec![RowId::new(103)]
    );

    let row_id_report = query_index
        .query_row_ids_with_stats_excluding_tombstones(&plan, &tombstones)
        .unwrap();
    assert_eq!(row_id_report.row_ids, vec![RowId::new(103)]);
    assert_eq!(row_id_report.stats.matched_records, 1);

    let rows = query_index
        .query_rows_excluding_tombstones(&plan, &tombstones)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].row_id(), RowId::new(103));

    let row_report = query_index
        .query_rows_with_stats_excluding_tombstones(&plan, &tombstones)
        .unwrap();
    assert_eq!(row_report.rows.len(), 1);
    assert_eq!(row_report.stats.matched_records, 1);

    assert_eq!(
        query_index
            .count_matches_excluding_tombstones(&plan, &tombstones)
            .unwrap(),
        1
    );
    assert_eq!(
        query_index
            .count_matches_with_stats_excluding_tombstones(&plan, &tombstones)
            .unwrap()
            .matched_records,
        1
    );
}

#[test]
fn typed_query_index_excludes_tombstoned_rows_from_visitors() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index(&schema);
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids(vec![RowId::new(103)]);
    let mut row_ids = Vec::new();
    let mut rows = Vec::new();

    let row_id_stats = query_index
        .visit_row_ids_excluding_tombstones(&plan, &tombstones, |row_id| {
            row_ids.push(row_id);
        })
        .unwrap();
    let row_stats = query_index
        .visit_rows_excluding_tombstones(&plan, &tombstones, |row_id, record| {
            rows.push((row_id, record.clone()));
        })
        .unwrap();

    assert_eq!(row_ids, vec![RowId::new(100)]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, RowId::new(100));
    assert_eq!(row_id_stats.matched_records, 1);
    assert_eq!(row_stats.matched_records, 1);
}

#[test]
fn typed_query_index_existence_ignores_tombstoned_matches() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index(&schema);
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids(vec![RowId::new(100), RowId::new(103)]);

    assert!(query_index.has_match(&plan).unwrap());
    assert!(
        !query_index
            .has_match_excluding_tombstones(&plan, &tombstones)
            .unwrap()
    );

    let report = query_index
        .has_match_with_stats_excluding_tombstones(&plan, &tombstones)
        .unwrap();
    assert!(!report.has_match);
    assert_eq!(report.stats.matched_records, 0);
    assert!(report.inspected_records > 0);
}

#[test]
fn typed_query_index_skips_tombstoned_missing_records() {
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
    let query_index =
        TypedQueryIndex::try_build(indexed_batch, &encoder, &builder()).expect("valid build");
    let query_index = TypedQueryIndex::from_parts(query_batch, query_index.index().clone());
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids(vec![RowId::new(103)]);

    assert_eq!(
        query_index.query_row_ids(&plan),
        Err(IndexedTypedQueryError::MissingRecord {
            row_id: RowId::new(103),
        })
    );
    assert_eq!(
        query_index
            .query_row_ids_excluding_tombstones(&plan, &tombstones)
            .unwrap(),
        vec![RowId::new(100)]
    );
}

fn typed_query_index(schema: &FSESchema) -> TypedQueryIndex {
    let batch = entity_batch(schema);
    let encoder = entity_encoder(schema);

    TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build")
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
