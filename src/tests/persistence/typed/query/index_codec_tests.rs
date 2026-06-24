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
    FSEFieldTypeArchiveTag, FSETypedQueryIndexArchiveCodecError,
    FSETypedQueryIndexArchiveSnapshot, FSEValueArchiveRecord,
    FSETypedQueryIndexArchiveSnapshotError, decode_typed_query_index_archive_snapshot,
    encode_typed_query_index_archive_snapshot, encode_typed_record_batch_archive_snapshot,
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
fn typed_query_index_archive_codec_compacts_categorical_record_batch_section() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let compact_archive_bytes = encode_typed_query_index_archive_snapshot(&snapshot).unwrap();
    let compact_batch_section_bytes = batch_section_bytes(&compact_archive_bytes);
    let old_record_batch_section_bytes =
        encode_typed_record_batch_archive_snapshot(&snapshot.batch).unwrap();
    let decoded = decode_typed_query_index_archive_snapshot(&compact_archive_bytes).unwrap();

    assert!(compact_batch_section_bytes.len() < old_record_batch_section_bytes.len());
    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_query_index_archive_codec_uses_compact_value_framing() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let compact_archive_bytes = encode_typed_query_index_archive_snapshot(&snapshot).unwrap();
    let compact_batch_section_bytes = batch_section_bytes(&compact_archive_bytes);
    let decoded = decode_typed_query_index_archive_snapshot(&compact_archive_bytes).unwrap();

    assert_eq!(compact_batch_section_bytes.len(), compact_entity_batch_section_len());
    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_query_index_archive_codec_decodes_legacy_compact_batch_section() {
    let snapshot =
        FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&typed_query_index()).unwrap();
    let archive_bytes = encode_typed_query_index_archive_snapshot(&snapshot).unwrap();
    let legacy_batch_section_bytes = legacy_compact_batch_section_bytes(&snapshot);
    let legacy_archive_bytes =
        replace_batch_section_bytes(&archive_bytes, &legacy_batch_section_bytes);
    let decoded = decode_typed_query_index_archive_snapshot(&legacy_archive_bytes).unwrap();

    assert_eq!(decoded, snapshot);
    assert!(legacy_batch_section_bytes.len() > batch_section_bytes(&archive_bytes).len());
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
    let first_field_type_offset = batch_payload_offset + 8 + 8 + 8 + "entity_id".len();
    bytes[first_field_type_offset] = 99;

    assert_eq!(
        decode_typed_query_index_archive_snapshot(&bytes),
        Err(
            FSETypedQueryIndexArchiveCodecError::InvalidCompactBatchFieldTypeTag {
                field: "typed_index.record_batch.schema.field.type",
                tag: 99,
            }
        )
    );
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

fn batch_section_bytes(bytes: &[u8]) -> &[u8] {
    let batch_length_offset = batch_length_offset(bytes);
    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(&bytes[batch_length_offset..batch_length_offset + 8]);
    let batch_length = u64::from_le_bytes(length_bytes) as usize;
    let batch_payload_offset = batch_length_offset + 8;

    &bytes[batch_payload_offset..batch_payload_offset + batch_length]
}

fn replace_batch_section_bytes(bytes: &[u8], batch_section: &[u8]) -> Vec<u8> {
    let batch_length_offset = batch_length_offset(bytes);
    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(&bytes[batch_length_offset..batch_length_offset + 8]);
    let old_batch_length = u64::from_le_bytes(length_bytes) as usize;
    let batch_payload_offset = batch_length_offset + 8;
    let old_batch_end = batch_payload_offset + old_batch_length;
    let mut replaced = Vec::new();

    replaced.extend_from_slice(&bytes[..batch_length_offset]);
    replaced.extend_from_slice(&(batch_section.len() as u64).to_le_bytes());
    replaced.extend_from_slice(batch_section);
    replaced.extend_from_slice(&bytes[old_batch_end..]);

    replaced
}

fn compact_entity_batch_section_len() -> usize {
    let schema_field_bytes =
        compact_field_len("entity_id") + compact_field_len("score") + compact_field_len("class")
            + compact_field_len("observed_at");
    let row_id_bytes = 8 + (4 * 8);
    let record_bytes = 4 * ((1 + 8) + (1 + 8) + (1 + 1) + (1 + 8));

    8 + 8 + schema_field_bytes + row_id_bytes + 8 + record_bytes
}

fn compact_field_len(name: &str) -> usize {
    8 + name.len() + 1 + 1
}

fn legacy_compact_batch_section_bytes(snapshot: &FSETypedQueryIndexArchiveSnapshot) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"FSECBT01");
    write_u64(&mut bytes, snapshot.batch.schema_fields.len() as u64);

    for field in &snapshot.batch.schema_fields {
        write_string(&mut bytes, &field.name);
        write_field_type_tag(&mut bytes, field.field_type);
        write_bool(&mut bytes, field.nullable);
    }

    write_u64_vec(&mut bytes, &snapshot.batch.row_ids);
    write_u64(&mut bytes, snapshot.batch.records.len() as u64);

    for record in &snapshot.batch.records {
        write_u64(&mut bytes, record.values.len() as u64);

        for value in &record.values {
            write_legacy_value(&mut bytes, value);
        }
    }

    bytes
}

fn write_legacy_value(bytes: &mut Vec<u8>, value: &FSEValueArchiveRecord) {
    match value {
        FSEValueArchiveRecord::Null => bytes.push(0),
        FSEValueArchiveRecord::Integer(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        FSEValueArchiveRecord::Float(value) => {
            bytes.push(2);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        FSEValueArchiveRecord::Text(value) => {
            bytes.push(3);
            write_string(bytes, value);
        }
        FSEValueArchiveRecord::Boolean(value) => {
            bytes.push(4);
            write_bool(bytes, *value);
        }
        FSEValueArchiveRecord::TimestampMillis(value) => {
            bytes.push(5);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        FSEValueArchiveRecord::Category(value) => {
            bytes.push(6);
            write_u64(bytes, category_code(value));
        }
    }
}

fn category_code(value: &str) -> u64 {
    match value {
        "alpha" => 0,
        "beta" => 1,
        other => panic!("unexpected test category: {other}"),
    }
}

fn write_field_type_tag(bytes: &mut Vec<u8>, tag: FSEFieldTypeArchiveTag) {
    bytes.push(match tag {
        FSEFieldTypeArchiveTag::Integer => 0,
        FSEFieldTypeArchiveTag::Float => 1,
        FSEFieldTypeArchiveTag::Text => 2,
        FSEFieldTypeArchiveTag::Boolean => 3,
        FSEFieldTypeArchiveTag::TimestampMillis => 4,
        FSEFieldTypeArchiveTag::Category => 5,
    });
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn write_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    write_u64(bytes, values.len() as u64);

    for value in values {
        write_u64(bytes, *value);
    }
}

fn write_bool(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
