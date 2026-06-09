use std::fs;
use std::io;
use std::path::PathBuf;

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
    FSEArchiveFileOperation, FSEArchivePayloadHeaderError, FSEArchivePayloadKind,
    FSETypedQueryIndexArchiveError, FSETypedQueryIndexArchiveFileError,
    FSETypedQueryIndexArchiveSnapshot, encode_archive_payload, load_typed_query_index_archive_file,
    read_typed_query_index_archive_snapshot_file, save_typed_query_index_archive_file,
    write_typed_query_index_archive_snapshot_file,
};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryPlanBuilder};

use super::corrupted_archive_payload;

#[test]
fn typed_query_index_archive_file_round_trips_snapshot_through_fse_file() {
    let query_index = typed_query_index();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    let path = temp_archive_path("snapshot-round-trip", ".fse");

    write_typed_query_index_archive_snapshot_file(&path, &snapshot).unwrap();
    let decoded = read_typed_query_index_archive_snapshot_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_methods_round_trip_snapshot() {
    let query_index = typed_query_index();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    let path = temp_archive_path("snapshot-methods", ".fse");

    snapshot.write_to_archive_file(&path).unwrap();
    let decoded = FSETypedQueryIndexArchiveSnapshot::read_from_archive_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_saves_and_loads_query_equivalent_index() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let path = temp_archive_path("index-round-trip", ".fse");

    save_typed_query_index_archive_file(&path, &query_index).unwrap();
    let loaded = load_typed_query_index_archive_file(&path).unwrap();

    assert_eq!(loaded.batch(), query_index.batch());
    assert_eq!(loaded.index(), query_index.index());
    assert_eq!(
        loaded.query_row_ids(&plan).unwrap(),
        query_index.query_row_ids(&plan).unwrap()
    );
    assert_eq!(
        loaded.query_rows(&plan).unwrap(),
        query_index.query_rows(&plan).unwrap()
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_reports_invalid_extension_on_save() {
    let query_index = typed_query_index();
    let path = temp_archive_path("wrong-save-extension", ".bin");

    assert_eq!(
        save_typed_query_index_archive_file(&path, &query_index),
        Err(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::InvalidFileExtension { path }
        ))
    );
}

#[test]
fn typed_query_index_archive_file_reports_missing_file_on_load() {
    let path = temp_archive_path("missing-load", ".fse");

    assert_eq!(
        load_typed_query_index_archive_file(&path),
        Err(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Read,
                path,
                kind: io::ErrorKind::NotFound
            }
        ))
    );
}

#[test]
fn typed_query_index_archive_file_reports_payload_header_errors_for_invalid_payload() {
    let path = temp_archive_path("invalid-payload", ".fse");

    fs::write(&path, [0_u8; 4]).unwrap();
    let error = read_typed_query_index_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSETypedQueryIndexArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedEndOfArchive { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_rejects_typed_record_batch_payload_kind() {
    let path = temp_archive_path("wrong-payload-kind", ".fse");
    let bytes = encode_archive_payload(FSEArchivePayloadKind::TypedRecordBatch, &[]);

    fs::write(&path, bytes).unwrap();

    assert_eq!(
        read_typed_query_index_archive_snapshot_file(&path),
        Err(FSETypedQueryIndexArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedPayloadKind {
                expected: FSEArchivePayloadKind::TypedQueryIndex,
                actual: FSEArchivePayloadKind::TypedRecordBatch
            }
        ))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_reports_payload_checksum_mismatch() {
    let path = temp_archive_path("checksum-mismatch", ".fse");
    let bytes = corrupted_archive_payload(FSEArchivePayloadKind::TypedQueryIndex);

    fs::write(&path, bytes).unwrap();
    let error = read_typed_query_index_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSETypedQueryIndexArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::PayloadChecksumMismatch { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

fn typed_query_index() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

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

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-typed-query-index-archive-file-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}
