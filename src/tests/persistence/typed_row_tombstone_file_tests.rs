use std::fs;
use std::io;
use std::path::PathBuf;

use crate::data::RowId;
use crate::persistence::{
    FSEArchiveFileOperation, FSEArchivePayloadHeaderError, FSEArchivePayloadKind,
    FSETypedRowTombstoneArchiveError, FSETypedRowTombstoneArchiveFileError,
    FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveSnapshotError,
    decode_archive_payload, encode_archive_payload, load_typed_row_tombstone_archive_file,
    read_typed_row_tombstone_archive_snapshot_file, save_typed_row_tombstone_archive_file,
    write_typed_row_tombstone_archive_snapshot_file,
};

use super::corrupted_archive_payload;

#[test]
fn typed_row_tombstone_archive_file_round_trips_snapshot_through_fse_file() {
    let snapshot = sample_snapshot();
    let path = temp_archive_path("snapshot-round-trip", ".fse");

    write_typed_row_tombstone_archive_snapshot_file(&path, &snapshot).unwrap();
    let decoded = read_typed_row_tombstone_archive_snapshot_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_row_tombstone_archive_file_methods_round_trip_snapshot() {
    let snapshot = sample_snapshot();
    let path = temp_archive_path("snapshot-methods", ".fse");

    snapshot.write_to_archive_file(&path).unwrap();
    let decoded = FSETypedRowTombstoneArchiveSnapshot::read_from_archive_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_row_tombstone_archive_file_saves_and_loads_row_ids() {
    let row_ids = vec![RowId::new(100), RowId::new(104), RowId::new(109)];
    let path = temp_archive_path("row-id-round-trip", ".fse");

    save_typed_row_tombstone_archive_file(&path, &row_ids).unwrap();
    let loaded = load_typed_row_tombstone_archive_file(&path).unwrap();

    assert_eq!(loaded, row_ids);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_row_tombstone_archive_file_reports_duplicate_row_ids_on_save() {
    let row_ids = vec![RowId::new(100), RowId::new(100)];
    let path = temp_archive_path("duplicate-save", ".fse");

    assert_eq!(
        save_typed_row_tombstone_archive_file(&path, &row_ids),
        Err(FSETypedRowTombstoneArchiveError::Snapshot(
            FSETypedRowTombstoneArchiveSnapshotError::DuplicateRowId { row_id: 100 },
        ))
    );
}

#[test]
fn typed_row_tombstone_archive_file_reports_duplicate_row_ids_after_read() {
    let path = temp_archive_path("duplicate-read", ".fse");
    let bytes = encode_archive_payload(
        FSEArchivePayloadKind::TypedRowTombstone,
        &tombstone_payload(&[100, 100]),
    );

    fs::write(&path, bytes).unwrap();

    assert_eq!(
        read_typed_row_tombstone_archive_snapshot_file(&path),
        Err(FSETypedRowTombstoneArchiveFileError::Codec(
            crate::persistence::FSETypedRowTombstoneArchiveCodecError::Snapshot(
                FSETypedRowTombstoneArchiveSnapshotError::DuplicateRowId { row_id: 100 },
            ),
        ))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_row_tombstone_archive_file_reports_invalid_extension_on_save() {
    let row_ids = vec![RowId::new(100)];
    let path = temp_archive_path("wrong-save-extension", ".bin");

    assert_eq!(
        save_typed_row_tombstone_archive_file(&path, &row_ids),
        Err(FSETypedRowTombstoneArchiveError::File(
            FSETypedRowTombstoneArchiveFileError::InvalidFileExtension { path },
        ))
    );
}

#[test]
fn typed_row_tombstone_archive_file_reports_missing_file_on_load() {
    let path = temp_archive_path("missing-load", ".fse");

    assert_eq!(
        load_typed_row_tombstone_archive_file(&path),
        Err(FSETypedRowTombstoneArchiveError::File(
            FSETypedRowTombstoneArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Read,
                path,
                kind: io::ErrorKind::NotFound,
            },
        ))
    );
}

#[test]
fn typed_row_tombstone_archive_file_rejects_typed_record_batch_payload_kind() {
    let path = temp_archive_path("wrong-payload-kind", ".fse");
    let bytes = encode_archive_payload(FSEArchivePayloadKind::TypedRecordBatch, &[]);

    fs::write(&path, bytes).unwrap();

    assert_eq!(
        read_typed_row_tombstone_archive_snapshot_file(&path),
        Err(FSETypedRowTombstoneArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedPayloadKind {
                expected: FSEArchivePayloadKind::TypedRowTombstone,
                actual: FSEArchivePayloadKind::TypedRecordBatch,
            },
        ))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_row_tombstone_archive_file_reports_payload_checksum_mismatch() {
    let path = temp_archive_path("checksum-mismatch", ".fse");
    let bytes = corrupted_archive_payload(FSEArchivePayloadKind::TypedRowTombstone);

    fs::write(&path, bytes).unwrap();
    let error = read_typed_row_tombstone_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSETypedRowTombstoneArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::PayloadChecksumMismatch { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn typed_row_tombstone_archive_file_exposes_payload_metadata() {
    let snapshot = sample_snapshot();
    let path = temp_archive_path("payload-metadata", ".fse");

    write_typed_row_tombstone_archive_snapshot_file(&path, &snapshot).unwrap();
    let bytes = fs::read(&path).unwrap();
    let payload = decode_archive_payload(FSEArchivePayloadKind::TypedRowTombstone, &bytes).unwrap();

    assert_eq!(payload, snapshot.to_archive_bytes().unwrap());

    let _ = fs::remove_file(path);
}

fn sample_snapshot() -> FSETypedRowTombstoneArchiveSnapshot {
    FSETypedRowTombstoneArchiveSnapshot::from_row_ids(vec![
        RowId::new(100),
        RowId::new(104),
        RowId::new(109),
    ])
    .unwrap()
}

fn tombstone_payload(row_ids: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(row_ids.len() as u64).to_le_bytes());

    for row_id in row_ids {
        bytes.extend_from_slice(&row_id.to_le_bytes());
    }

    bytes
}

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-typed-row-tombstone-archive-file-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}
