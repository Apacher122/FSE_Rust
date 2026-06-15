use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FSEFieldEncoderMetadata,
    FSERecordEncoderMetadata, FloatEncoder, IntegerEncoder, TimestampMillisEncoder,
};
use crate::persistence::{
    FSETypedQueryIndexArchiveCodecError, FSETypedQueryIndexArchiveSnapshot,
    FSETypedQueryIndexArchiveSnapshotError, decode_typed_query_index_archive_snapshot,
    encode_typed_query_index_archive_snapshot,
};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryPlanBuilder};

#[test]
fn typed_query_index_archive_codec_round_trips_snapshot() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let bytes = encode_typed_query_index_archive_snapshot(&snapshot).unwrap();
    let decoded = decode_typed_query_index_archive_snapshot(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_query_index_archive_codec_round_trips_record_encoder_metadata() {
    let query_index = typed_query_index_with_reverse_category_encoder();
    let metadata = reverse_category_encoder_metadata();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index_with_encoder_metadata(
        &query_index,
        metadata.clone(),
    )
    .unwrap();

    let bytes = encode_typed_query_index_archive_snapshot(&snapshot).unwrap();
    let decoded = decode_typed_query_index_archive_snapshot(&bytes).unwrap();

    assert_eq!(decoded.record_encoder, metadata);
    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_query_index_archive_codec_methods_round_trip_snapshot() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSETypedQueryIndexArchiveSnapshot::from_archive_bytes(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_query_index_archive_codec_rebuilds_query_equivalent_index_after_decode() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSETypedQueryIndexArchiveSnapshot::from_archive_bytes(&bytes).unwrap();
    let rebuilt = decoded.to_typed_query_index().unwrap();
    let plan = score_and_class_plan(&schema, &mapping);

    assert_eq!(
        rebuilt.query_row_ids(&plan).unwrap(),
        query_index.query_row_ids(&plan).unwrap()
    );
    assert_eq!(
        rebuilt.count_matches(&plan).unwrap(),
        query_index.count_matches(&plan).unwrap()
    );
}

#[test]
fn typed_query_index_archive_codec_rejects_invalid_snapshot_on_encode() {
    let mut snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    snapshot.batch.row_ids[3] = 999;

    assert_eq!(
        encode_typed_query_index_archive_snapshot(&snapshot),
        Err(FSETypedQueryIndexArchiveCodecError::Snapshot(
            FSETypedQueryIndexArchiveSnapshotError::RowIdMismatch {
                indexed_row_id: 103,
                batch_row_id: 999,
            },
        ))
    );
}

#[test]
fn typed_query_index_archive_codec_rejects_truncated_bytes() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    bytes.pop();

    assert!(matches!(
        decode_typed_query_index_archive_snapshot(&bytes),
        Err(FSETypedQueryIndexArchiveCodecError::UnexpectedEndOfArchive { .. })
    ));
}

#[test]
fn typed_query_index_archive_codec_rejects_trailing_bytes() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    bytes.push(99);

    assert_eq!(
        decode_typed_query_index_archive_snapshot(&bytes),
        Err(FSETypedQueryIndexArchiveCodecError::TrailingBytes { remaining: 1 })
    );
}

#[test]
fn typed_query_index_archive_codec_reports_embedded_index_codec_error() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    let index_payload_offset = 8;
    bytes[index_payload_offset..index_payload_offset + 8].copy_from_slice(&4_u64.to_le_bytes());

    assert!(matches!(
        decode_typed_query_index_archive_snapshot(&bytes),
        Err(FSETypedQueryIndexArchiveCodecError::IndexCodec(_))
    ));
}

#[test]
fn typed_query_index_archive_codec_reports_embedded_batch_codec_error() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    let batch_length_offset = batch_length_offset(&bytes);
    let batch_payload_offset = batch_length_offset + 8;
    let first_field_type_offset = batch_payload_offset + 8 + 8 + "entity_id".len();
    bytes[first_field_type_offset] = 99;

    assert!(matches!(
        decode_typed_query_index_archive_snapshot(&bytes),
        Err(FSETypedQueryIndexArchiveCodecError::BatchCodec(_))
    ));
}

#[test]
fn typed_query_index_archive_codec_reports_invalid_encoder_metadata_tag() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    let metadata_payload_offset = record_encoder_metadata_payload_offset(&bytes);
    let first_field_tag_offset = metadata_payload_offset + 8;
    bytes[first_field_tag_offset] = 99;

    assert_eq!(
        decode_typed_query_index_archive_snapshot(&bytes),
        Err(
            FSETypedQueryIndexArchiveCodecError::InvalidFieldEncoderMetadataTag {
                field: "typed_index.record_encoder.field",
                tag: 99,
            }
        )
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

fn batch_length_offset(bytes: &[u8]) -> usize {
    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(&bytes[0..8]);
    let index_length = u64::from_le_bytes(length_bytes) as usize;

    8 + index_length
}

fn record_encoder_metadata_payload_offset(bytes: &[u8]) -> usize {
    let batch_length_offset = batch_length_offset(bytes);
    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(&bytes[batch_length_offset..batch_length_offset + 8]);
    let batch_length = u64::from_le_bytes(length_bytes) as usize;

    batch_length_offset + 8 + batch_length + 8
}
