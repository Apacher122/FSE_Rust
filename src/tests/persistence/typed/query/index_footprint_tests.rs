use std::fs;
use std::io;
use std::path::PathBuf;

use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema, FSEValue, RowId};
use crate::encoding::{ComposedRecordEncoder, FloatEncoder, IntegerEncoder};
use crate::persistence::{
    FSEArchiveFileOperation, FSETypedQueryIndexArchiveFootprint,
    FSETypedQueryIndexArchiveFootprintComponent, FSETypedQueryIndexArchiveFootprintError,
    save_typed_query_index_archive_file, save_typed_record_batch_archive_file,
    save_typed_row_tombstone_archive_file, typed_query_index_archive_footprint,
    typed_query_index_archive_with_append_delta_and_tombstones_footprint,
    typed_query_index_archive_with_append_delta_footprint,
    typed_query_index_archive_with_tombstones_footprint,
};
use crate::query::TypedQueryIndex;

#[test]
fn typed_query_index_archive_footprint_counts_query_index_file_bytes() {
    let path = temp_archive_path("query-only", ".fse");

    save_typed_query_index_archive_file(&path, &typed_query_index()).unwrap();

    let expected_query_index_bytes = fs::metadata(&path).unwrap().len();
    let footprint = typed_query_index_archive_footprint(&path).unwrap();

    assert_eq!(
        footprint.query_index_archive_bytes,
        expected_query_index_bytes
    );
    assert_eq!(footprint.append_delta_archive_bytes, 0);
    assert_eq!(footprint.tombstone_archive_bytes, 0);
    assert_eq!(footprint.total_archive_bytes, expected_query_index_bytes);
    assert!(!footprint.includes_append_delta_archive());
    assert!(!footprint.includes_tombstone_archive());

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_footprint_counts_append_delta_file_bytes() {
    let query_index_path = temp_archive_path("with-append-index", ".fse");
    let append_delta_path = temp_archive_path("with-append-delta", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &typed_query_index()).unwrap();
    save_typed_record_batch_archive_file(&append_delta_path, &append_record_batch()).unwrap();

    let expected_query_index_bytes = fs::metadata(&query_index_path).unwrap().len();
    let expected_append_delta_bytes = fs::metadata(&append_delta_path).unwrap().len();
    let footprint = typed_query_index_archive_with_append_delta_footprint(
        &query_index_path,
        &append_delta_path,
    )
    .unwrap();

    assert_eq!(
        footprint.query_index_archive_bytes,
        expected_query_index_bytes
    );
    assert_eq!(
        footprint.append_delta_archive_bytes,
        expected_append_delta_bytes
    );
    assert_eq!(footprint.tombstone_archive_bytes, 0);
    assert_eq!(
        footprint.total_archive_bytes,
        expected_query_index_bytes + expected_append_delta_bytes
    );
    assert!(footprint.includes_append_delta_archive());
    assert!(!footprint.includes_tombstone_archive());

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(append_delta_path);
}

#[test]
fn typed_query_index_archive_footprint_counts_tombstone_file_bytes() {
    let query_index_path = temp_archive_path("with-tombstones-index", ".fse");
    let tombstone_path = temp_archive_path("with-tombstones-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &typed_query_index()).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(100)]).unwrap();

    let expected_query_index_bytes = fs::metadata(&query_index_path).unwrap().len();
    let expected_tombstone_bytes = fs::metadata(&tombstone_path).unwrap().len();
    let footprint =
        typed_query_index_archive_with_tombstones_footprint(&query_index_path, &tombstone_path)
            .unwrap();

    assert_eq!(
        footprint.query_index_archive_bytes,
        expected_query_index_bytes
    );
    assert_eq!(footprint.append_delta_archive_bytes, 0);
    assert_eq!(footprint.tombstone_archive_bytes, expected_tombstone_bytes);
    assert_eq!(
        footprint.total_archive_bytes,
        expected_query_index_bytes + expected_tombstone_bytes
    );
    assert!(!footprint.includes_append_delta_archive());
    assert!(footprint.includes_tombstone_archive());

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_footprint_counts_all_component_file_bytes() {
    let query_index_path = temp_archive_path("with-all-index", ".fse");
    let append_delta_path = temp_archive_path("with-all-append-delta", ".fse");
    let tombstone_path = temp_archive_path("with-all-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &typed_query_index()).unwrap();
    save_typed_record_batch_archive_file(&append_delta_path, &append_record_batch()).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(100)]).unwrap();

    let expected_query_index_bytes = fs::metadata(&query_index_path).unwrap().len();
    let expected_append_delta_bytes = fs::metadata(&append_delta_path).unwrap().len();
    let expected_tombstone_bytes = fs::metadata(&tombstone_path).unwrap().len();
    let footprint = typed_query_index_archive_with_append_delta_and_tombstones_footprint(
        &query_index_path,
        &append_delta_path,
        &tombstone_path,
    )
    .unwrap();

    assert_eq!(
        footprint.query_index_archive_bytes,
        expected_query_index_bytes
    );
    assert_eq!(
        footprint.append_delta_archive_bytes,
        expected_append_delta_bytes
    );
    assert_eq!(footprint.tombstone_archive_bytes, expected_tombstone_bytes);
    assert_eq!(
        footprint.total_archive_bytes,
        expected_query_index_bytes + expected_append_delta_bytes + expected_tombstone_bytes
    );
    assert!(footprint.includes_append_delta_archive());
    assert!(footprint.includes_tombstone_archive());

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(append_delta_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_footprint_reports_missing_query_index_file() {
    let path = temp_archive_path("missing-query-index", ".fse");

    assert_eq!(
        typed_query_index_archive_footprint(&path),
        Err(FSETypedQueryIndexArchiveFootprintError::Io {
            component: FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
            operation: FSEArchiveFileOperation::Read,
            path,
            kind: io::ErrorKind::NotFound,
        })
    );
}

#[test]
fn typed_query_index_archive_footprint_reports_invalid_append_delta_extension() {
    let query_index_path = temp_archive_path("invalid-append-extension-index", ".fse");
    let append_delta_path = temp_archive_path("invalid-append-extension", ".bin");

    save_typed_query_index_archive_file(&query_index_path, &typed_query_index()).unwrap();

    assert_eq!(
        typed_query_index_archive_with_append_delta_footprint(
            &query_index_path,
            &append_delta_path
        ),
        Err(
            FSETypedQueryIndexArchiveFootprintError::InvalidFileExtension {
                component: FSETypedQueryIndexArchiveFootprintComponent::AppendDelta,
                path: append_delta_path
            }
        )
    );

    let _ = fs::remove_file(query_index_path);
}

#[test]
fn typed_query_index_archive_footprint_reports_invalid_tombstone_extension() {
    let query_index_path = temp_archive_path("invalid-tombstone-extension-index", ".fse");
    let tombstone_path = temp_archive_path("invalid-tombstone-extension", ".bin");

    save_typed_query_index_archive_file(&query_index_path, &typed_query_index()).unwrap();

    assert_eq!(
        typed_query_index_archive_with_tombstones_footprint(&query_index_path, &tombstone_path),
        Err(
            FSETypedQueryIndexArchiveFootprintError::InvalidFileExtension {
                component: FSETypedQueryIndexArchiveFootprintComponent::Tombstones,
                path: tombstone_path
            }
        )
    );

    let _ = fs::remove_file(query_index_path);
}

#[test]
fn typed_query_index_archive_footprint_reports_total_byte_count_overflow() {
    assert_eq!(
        FSETypedQueryIndexArchiveFootprint::try_new(u64::MAX, 1),
        Err(
            FSETypedQueryIndexArchiveFootprintError::TotalArchiveByteCountOverflow {
                query_index_archive_bytes: u64::MAX,
                append_delta_archive_bytes: 0,
                tombstone_archive_bytes: 1,
            }
        )
    );

    assert_eq!(
        FSETypedQueryIndexArchiveFootprint::try_new_with_append_delta(u64::MAX, 1, 0),
        Err(
            FSETypedQueryIndexArchiveFootprintError::TotalArchiveByteCountOverflow {
                query_index_archive_bytes: u64::MAX,
                append_delta_archive_bytes: 1,
                tombstone_archive_bytes: 0,
            }
        )
    );
}

fn typed_query_index() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = base_record_batch();
    let encoder = ComposedRecordEncoder::new(
        &schema,
        vec![Box::new(IntegerEncoder), Box::new(FloatEncoder)],
    );
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    TypedQueryIndex::try_build(batch, &encoder, &builder).unwrap()
}

fn base_record_batch() -> FSERecordBatch {
    let schema = entity_schema();

    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100), RowId::new(101)],
        vec![
            FSERecord::new(vec![FSEValue::Integer(100), FSEValue::Float(12.5)], &schema),
            FSERecord::new(vec![FSEValue::Integer(101), FSEValue::Float(25.0)], &schema),
        ],
    )
}

fn append_record_batch() -> FSERecordBatch {
    let schema = entity_schema();

    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(102)],
        vec![FSERecord::new(
            vec![FSEValue::Integer(102), FSEValue::Float(37.5)],
            &schema,
        )],
    )
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
    ])
}

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-typed-query-index-archive-footprint-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}
