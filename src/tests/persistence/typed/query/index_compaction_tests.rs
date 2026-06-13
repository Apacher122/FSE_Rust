use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::persistence::{
    FSETombstonedTypedQueryIndex, FSETypedQueryIndexCompactionError,
    compact_tombstoned_typed_query_index,
};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryPlanBuilder};

#[test]
fn typed_query_index_compaction_rebuilds_live_index_with_same_logical_results() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstoned = FSETombstonedTypedQueryIndex::from_row_ids(query_index, [RowId::new(100)]);

    let expected = tombstoned.query_row_ids(&plan).unwrap();
    let result = compact_tombstoned_typed_query_index(&tombstoned, &encoder, &builder).unwrap();

    assert_eq!(expected, vec![RowId::new(103)]);
    assert_eq!(result.base_record_count, 4);
    assert_eq!(result.tombstone_count, 1);
    assert_eq!(result.removed_record_count, 1);
    assert_eq!(result.retained_record_count, 3);
    assert_eq!(result.query_index.query_row_ids(&plan).unwrap(), expected);
    assert_eq!(
        result.query_index.batch().row_ids(),
        &[RowId::new(101), RowId::new(102), RowId::new(103)]
    );
    assert!(
        result
            .query_index
            .batch()
            .record_for_row_id(RowId::new(100))
            .is_none()
    );
    assert_eq!(
        result.query_index.index().index().root_node().cardinality,
        3
    );
}

#[test]
fn typed_query_index_compaction_method_matches_free_function() {
    let encoder = entity_encoder(&entity_schema());
    let builder = builder();
    let query_index = typed_query_index();
    let tombstoned = FSETombstonedTypedQueryIndex::from_row_ids(query_index, [RowId::new(101)]);

    let from_method = tombstoned.compact(&encoder, &builder).unwrap();
    let from_function =
        compact_tombstoned_typed_query_index(&tombstoned, &encoder, &builder).unwrap();

    assert_eq!(from_method, from_function);
}

#[test]
fn typed_query_index_compaction_counts_only_tombstones_present_in_base_index() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstoned = FSETombstonedTypedQueryIndex::from_row_ids(
        query_index,
        [RowId::new(999), RowId::new(1_000)],
    );

    let result = tombstoned.compact(&encoder, &builder).unwrap();

    assert_eq!(result.base_record_count, 4);
    assert_eq!(result.tombstone_count, 2);
    assert_eq!(result.removed_record_count, 0);
    assert_eq!(result.retained_record_count, 4);
    assert_eq!(
        result.query_index.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );
}

#[test]
fn typed_query_index_compaction_reports_empty_retained_record_set() {
    let encoder = entity_encoder(&entity_schema());
    let builder = builder();
    let query_index = typed_query_index();
    let tombstoned = FSETombstonedTypedQueryIndex::from_row_ids(
        query_index,
        [
            RowId::new(100),
            RowId::new(101),
            RowId::new(102),
            RowId::new(103),
        ],
    );

    assert_eq!(
        tombstoned.compact(&encoder, &builder),
        Err(FSETypedQueryIndexCompactionError::EmptyRetainedRecordSet {
            base_record_count: 4,
            tombstone_count: 4,
        })
    );
}

fn typed_query_index() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();

    TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid typed index should build")
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
