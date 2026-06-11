use std::fs;
use std::io;
use std::path::PathBuf;

use crate::data::{
    FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError, FSESchema, FSEValue,
    RowId,
};
use crate::persistence::{
    FSEArchiveAppendOperationMetadataError, FSEArchiveFileOperation, FSEArchivePayloadHeaderError,
    FSEArchivePayloadKind, FSEArchiveRebuildReason, FSERecordBatchArchiveError,
    FSERecordBatchArchiveFileError, FSERecordBatchArchiveSnapshot,
    append_typed_record_batch_archive_file, encode_archive_payload,
    load_typed_record_batch_archive_file, read_typed_record_batch_archive_snapshot_file,
    save_typed_record_batch_archive_file, write_typed_record_batch_archive_snapshot_file,
};

use super::corrupted_archive_payload;

#[test]
fn typed_record_batch_archive_file_round_trips_snapshot_through_fse_file() {
    let batch = sample_batch();
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    let path = temp_archive_path("snapshot-round-trip", ".fse");

    write_typed_record_batch_archive_snapshot_file(&path, &snapshot).unwrap();
    let decoded = read_typed_record_batch_archive_snapshot_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_methods_round_trip_snapshot() {
    let batch = sample_batch();
    let snapshot = FSERecordBatchArchiveSnapshot::from_record_batch(&batch);
    let path = temp_archive_path("snapshot-methods", ".fse");

    snapshot.write_to_archive_file(&path).unwrap();
    let decoded = FSERecordBatchArchiveSnapshot::read_from_archive_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_saves_and_loads_record_batch() {
    let batch = sample_batch();
    let path = temp_archive_path("batch-round-trip", ".fse");

    save_typed_record_batch_archive_file(&path, &batch).unwrap();
    let loaded = load_typed_record_batch_archive_file(&path).unwrap();

    assert_eq!(loaded, batch);
    assert_eq!(loaded.row_ids(), &[RowId::new(100), RowId::new(101)]);
    assert_eq!(
        loaded.record_for_row_id(RowId::new(101)).unwrap().value(0),
        Some(&FSEValue::Integer(2))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_appends_batch_and_rebuilds_archive() {
    let base = sample_batch();
    let appended = sample_appended_batch();
    let path = temp_archive_path("batch-append", ".fse");

    save_typed_record_batch_archive_file(&path, &base).unwrap();
    let result = append_typed_record_batch_archive_file(&path, &appended).unwrap();
    let loaded = load_typed_record_batch_archive_file(&path).unwrap();

    assert_eq!(
        result.append_metadata.payload_kind,
        FSEArchivePayloadKind::TypedRecordBatch
    );
    assert_eq!(result.append_metadata.base_record_count, 2);
    assert_eq!(result.append_metadata.appended_record_count, 2);
    assert_eq!(result.append_metadata.resulting_record_count, 4);
    assert_eq!(result.rebuild_plan.reason, FSEArchiveRebuildReason::Append);
    assert!(result.rebuild_plan.requires_full_archive_rebuild);
    assert_eq!(result.record_batch, loaded);
    assert_eq!(
        loaded.row_ids(),
        &[
            RowId::new(100),
            RowId::new(101),
            RowId::new(102),
            RowId::new(103)
        ]
    );
    assert_eq!(
        loaded.record_for_row_id(RowId::new(103)).unwrap().value(0),
        Some(&FSEValue::Integer(4))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_reports_append_schema_mismatch() {
    let base = sample_batch();
    let appended = mismatched_appended_batch();
    let path = temp_archive_path("append-schema-mismatch", ".fse");

    save_typed_record_batch_archive_file(&path, &base).unwrap();

    assert_eq!(
        append_typed_record_batch_archive_file(&path, &appended),
        Err(FSERecordBatchArchiveError::RecordBatch(
            FSERecordBatchError::SchemaMismatch
        ))
    );
    assert_eq!(load_typed_record_batch_archive_file(&path).unwrap(), base);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_reports_empty_append_batch() {
    let base = sample_batch();
    let appended = FSERecordBatch::new(sample_schema(), Vec::new(), Vec::new());
    let path = temp_archive_path("empty-append", ".fse");

    save_typed_record_batch_archive_file(&path, &base).unwrap();

    assert_eq!(
        append_typed_record_batch_archive_file(&path, &appended),
        Err(FSERecordBatchArchiveError::RecordBatch(
            FSERecordBatchError::EmptyAppendBatch
        ))
    );
    assert_eq!(load_typed_record_batch_archive_file(&path).unwrap(), base);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_reports_zero_base_record_count_for_append() {
    let base = FSERecordBatch::new(sample_schema(), Vec::new(), Vec::new());
    let appended = sample_appended_batch();
    let path = temp_archive_path("zero-base-append", ".fse");

    save_typed_record_batch_archive_file(&path, &base).unwrap();

    assert_eq!(
        append_typed_record_batch_archive_file(&path, &appended),
        Err(FSERecordBatchArchiveError::AppendMetadata(
            FSEArchiveAppendOperationMetadataError::ZeroBaseRecordCount
        ))
    );
    assert_eq!(load_typed_record_batch_archive_file(&path).unwrap(), base);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_reports_invalid_extension_on_save() {
    let batch = sample_batch();
    let path = temp_archive_path("wrong-save-extension", ".bin");

    assert_eq!(
        save_typed_record_batch_archive_file(&path, &batch),
        Err(FSERecordBatchArchiveError::File(
            FSERecordBatchArchiveFileError::InvalidFileExtension { path }
        ))
    );
}

#[test]
fn typed_record_batch_archive_file_reports_missing_file_on_load() {
    let path = temp_archive_path("missing-load", ".fse");

    assert_eq!(
        load_typed_record_batch_archive_file(&path),
        Err(FSERecordBatchArchiveError::File(
            FSERecordBatchArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Read,
                path,
                kind: io::ErrorKind::NotFound
            }
        ))
    );
}

#[test]
fn typed_record_batch_archive_file_reports_payload_header_errors_for_invalid_payload() {
    let path = temp_archive_path("invalid-payload", ".fse");

    fs::write(&path, [0_u8; 4]).unwrap();
    let error = read_typed_record_batch_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSERecordBatchArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedEndOfArchive { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_rejects_typed_query_index_payload_kind() {
    let path = temp_archive_path("wrong-payload-kind", ".fse");
    let bytes = encode_archive_payload(FSEArchivePayloadKind::TypedQueryIndex, &[]);

    fs::write(&path, bytes).unwrap();

    assert_eq!(
        read_typed_record_batch_archive_snapshot_file(&path),
        Err(FSERecordBatchArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedPayloadKind {
                expected: FSEArchivePayloadKind::TypedRecordBatch,
                actual: FSEArchivePayloadKind::TypedQueryIndex
            }
        ))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_record_batch_archive_file_reports_payload_checksum_mismatch() {
    let path = temp_archive_path("checksum-mismatch", ".fse");
    let bytes = corrupted_archive_payload(FSEArchivePayloadKind::TypedRecordBatch);

    fs::write(&path, bytes).unwrap();
    let error = read_typed_record_batch_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSERecordBatchArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::PayloadChecksumMismatch { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

fn sample_batch() -> FSERecordBatch {
    let schema = sample_schema();
    let records = vec![
        sample_record(
            1,
            12.5,
            "alpha",
            true,
            1_735_689_600_000,
            "open",
            FSEValue::Text("reviewed".to_string()),
            &schema,
        ),
        sample_record(
            2,
            24.0,
            "beta",
            false,
            1_735_776_000_000,
            "closed",
            FSEValue::Null,
            &schema,
        ),
    ];

    FSERecordBatch::new(schema, vec![RowId::new(100), RowId::new(101)], records)
}

fn sample_appended_batch() -> FSERecordBatch {
    let schema = sample_schema();
    let records = vec![
        sample_record(
            3,
            36.5,
            "gamma",
            true,
            1_735_862_400_000,
            "review",
            FSEValue::Text("queued".to_string()),
            &schema,
        ),
        sample_record(
            4,
            48.0,
            "delta",
            false,
            1_735_948_800_000,
            "closed",
            FSEValue::Null,
            &schema,
        ),
    ];

    FSERecordBatch::new(schema, vec![RowId::new(102), RowId::new(103)], records)
}

fn mismatched_appended_batch() -> FSERecordBatch {
    let schema = FSESchema::new(vec![
        FSEField::new("record_id", FSEFieldType::Integer, false),
        FSEField::new("amount", FSEFieldType::Float, false),
    ]);
    let record = FSERecord::new(vec![FSEValue::Integer(3), FSEValue::Float(36.5)], &schema);

    FSERecordBatch::new(schema, vec![RowId::new(102)], vec![record])
}

fn sample_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("record_id", FSEFieldType::Integer, false),
        FSEField::new("amount", FSEFieldType::Float, false),
        FSEField::new("label", FSEFieldType::Text, false),
        FSEField::new("active", FSEFieldType::Boolean, false),
        FSEField::new("created_at", FSEFieldType::TimestampMillis, false),
        FSEField::new("status", FSEFieldType::Category, false),
        FSEField::new("notes", FSEFieldType::Text, true),
    ])
}

fn sample_record(
    record_id: i64,
    amount: f64,
    label: &str,
    active: bool,
    created_at: i64,
    status: &str,
    notes: FSEValue,
    schema: &FSESchema,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(record_id),
            FSEValue::Float(amount),
            FSEValue::Text(label.to_string()),
            FSEValue::Boolean(active),
            FSEValue::TimestampMillis(created_at),
            FSEValue::Category(status.to_string()),
            notes,
        ],
        schema,
    )
}

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-typed-record-batch-archive-file-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}
