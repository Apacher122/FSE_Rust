use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FSEFieldEncoderMetadata,
    FSERecordEncoderMetadata, FSERecordEncoderMetadataError, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::persistence::{
    FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError,
};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryPlanBuilder};

#[test]
fn typed_query_index_archive_snapshot_captures_index_and_batch() {
    let query_index = typed_query_index();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();

    assert_eq!(
        snapshot.index.index.manifest.record_count,
        query_index.batch().len() as u64
    );
    assert_eq!(snapshot.batch.row_ids, vec![100, 101, 102, 103]);
    assert_eq!(
        snapshot.record_encoder.fields(),
        &[
            FSEFieldEncoderMetadata::Integer,
            FSEFieldEncoderMetadata::Float,
            FSEFieldEncoderMetadata::CategoryDictionary {
                categories: vec!["alpha".to_string(), "beta".to_string()],
            },
            FSEFieldEncoderMetadata::TimestampMillis,
        ]
    );
    assert_eq!(snapshot.index.leaf_row_id_records.len(), 2);
    assert!(snapshot.validate().is_ok());
}

#[test]
fn typed_query_index_archive_snapshot_accepts_explicit_record_encoder_metadata() {
    let query_index = typed_query_index_with_reverse_category_encoder();
    let metadata = reverse_category_encoder_metadata();

    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index_with_encoder_metadata(
        &query_index,
        metadata.clone(),
    )
    .unwrap();

    assert_eq!(snapshot.record_encoder, metadata);
    assert!(snapshot.validate().is_ok());
}

#[test]
fn typed_query_index_archive_snapshot_rebuilds_query_equivalent_index() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    let rebuilt = snapshot.to_typed_query_index().unwrap();
    let plan = score_and_class_plan(&schema, &mapping);

    assert_eq!(rebuilt.batch(), query_index.batch());
    assert_eq!(rebuilt.index(), query_index.index());
    assert_eq!(
        rebuilt.query_row_ids(&plan).unwrap(),
        query_index.query_row_ids(&plan).unwrap()
    );
    assert_eq!(
        rebuilt.query_rows(&plan).unwrap(),
        query_index.query_rows(&plan).unwrap()
    );
}

#[test]
fn typed_query_index_archive_snapshot_reports_row_id_count_mismatch() {
    let query_index = typed_query_index();
    let mut snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    snapshot.batch.row_ids.pop();
    snapshot.batch.records.pop();

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedQueryIndexArchiveSnapshotError::RowIdCountMismatch {
            indexed_row_id_count: 4,
            batch_row_id_count: 3,
        })
    );
}

#[test]
fn typed_query_index_archive_snapshot_reports_row_id_mismatch() {
    let query_index = typed_query_index();
    let mut snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    snapshot.batch.row_ids[3] = 999;

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedQueryIndexArchiveSnapshotError::RowIdMismatch {
            indexed_row_id: 103,
            batch_row_id: 999,
        })
    );
}

#[test]
fn typed_query_index_archive_snapshot_reports_record_encoder_metadata_mismatch() {
    let query_index = typed_query_index();
    let mut snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    snapshot.record_encoder = FSERecordEncoderMetadata::new(vec![
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::CategoryDictionary {
            categories: vec!["alpha".to_string(), "beta".to_string()],
        },
        FSEFieldEncoderMetadata::TimestampMillis,
    ]);

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedQueryIndexArchiveSnapshotError::EncoderMetadata(
            FSERecordEncoderMetadataError::FieldTypeMismatch {
                field: 1,
                name: "score".to_string(),
                expected: FSEFieldType::Float,
                actual: FSEFieldType::Integer,
            }
        ))
    );
}

fn typed_query_index() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid typed index should build")
}

fn typed_query_index_with_reverse_category_encoder() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = ComposedRecordEncoder::new(
        &schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(FloatEncoder),
            Box::new(CategoricalDictionaryEncoder::new(vec![
                "beta".to_string(),
                "alpha".to_string(),
            ])),
            Box::new(TimestampMillisEncoder),
        ],
    );
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid typed index should build")
}

fn reverse_category_encoder_metadata() -> FSERecordEncoderMetadata {
    FSERecordEncoderMetadata::new(vec![
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::Float,
        FSEFieldEncoderMetadata::CategoryDictionary {
            categories: vec!["beta".to_string(), "alpha".to_string()],
        },
        FSEFieldEncoderMetadata::TimestampMillis,
    ])
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
