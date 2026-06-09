use std::fs;
use std::io;
use std::path::PathBuf;

use crate::build::{BuildConfig, FSEBuilder};
use crate::data::RowId;
use crate::encoding::EncodedRecordBatch;
use crate::math::Vector;
use crate::persistence::{
    FSEArchiveFileOperation, FSEArchivePayloadHeaderError, FSEArchivePayloadKind,
    FSERowMappedArchiveFileError, FSERowMappedIndexArchiveError, FSERowMappedIndexArchiveSnapshot,
    encode_archive_payload, load_row_mapped_index_archive_file,
    read_row_mapped_archive_snapshot_file, save_row_mapped_index_archive_file,
    write_row_mapped_archive_snapshot_file,
};

use super::corrupted_archive_payload;

fn row_mapped_fixture() -> crate::build::RowMappedFSEIndex {
    let encoded = EncodedRecordBatch::new(
        vec![
            RowId::new(10),
            RowId::new(11),
            RowId::new(12),
            RowId::new(13),
        ],
        vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![1.0, 0.0]),
            Vector::new(vec![50.0, 0.0]),
            Vector::new(vec![51.0, 0.0]),
        ],
    );
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    builder
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("valid encoded batch should build")
}

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-row-mapped-archive-file-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn row_mapped_archive_file_round_trips_snapshot_through_fse_file() {
    let mapped = row_mapped_fixture();
    let snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    let path = temp_archive_path("snapshot-round-trip", ".fse");

    write_row_mapped_archive_snapshot_file(&path, &snapshot).unwrap();
    let decoded = read_row_mapped_archive_snapshot_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn row_mapped_archive_file_methods_round_trip_snapshot() {
    let mapped = row_mapped_fixture();
    let snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    let path = temp_archive_path("snapshot-methods", ".fse");

    snapshot.write_to_archive_file(&path).unwrap();
    let decoded = FSERowMappedIndexArchiveSnapshot::read_from_archive_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn row_mapped_index_archive_file_saves_and_loads_row_mapping() {
    let mapped = row_mapped_fixture();
    let path = temp_archive_path("index-round-trip", ".fse");

    save_row_mapped_index_archive_file(&path, &mapped).unwrap();
    let loaded = load_row_mapped_index_archive_file(&path).unwrap();

    assert_eq!(loaded.index(), mapped.index());
    assert_eq!(
        loaded.leaf_row_ids(1),
        Some([RowId::new(10), RowId::new(11)].as_slice())
    );
    assert_eq!(
        loaded.leaf_row_ids(2),
        Some([RowId::new(12), RowId::new(13)].as_slice())
    );
    assert_eq!(loaded.leaf_row_ids(0), None);

    let _ = fs::remove_file(path);
}

#[test]
fn row_mapped_index_archive_file_reports_invalid_extension_on_save() {
    let mapped = row_mapped_fixture();
    let path = temp_archive_path("wrong-save-extension", ".bin");

    assert_eq!(
        save_row_mapped_index_archive_file(&path, &mapped),
        Err(FSERowMappedIndexArchiveError::File(
            FSERowMappedArchiveFileError::InvalidFileExtension { path }
        ))
    );
}

#[test]
fn row_mapped_index_archive_file_reports_missing_file_on_load() {
    let path = temp_archive_path("missing-load", ".fse");

    assert_eq!(
        load_row_mapped_index_archive_file(&path),
        Err(FSERowMappedIndexArchiveError::File(
            FSERowMappedArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Read,
                path,
                kind: io::ErrorKind::NotFound
            }
        ))
    );
}

#[test]
fn row_mapped_archive_file_reports_payload_header_errors_for_invalid_payload() {
    let path = temp_archive_path("invalid-payload", ".fse");

    fs::write(&path, [0_u8; 4]).unwrap();
    let error = read_row_mapped_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSERowMappedArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedEndOfArchive { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn row_mapped_archive_file_rejects_index_payload_kind() {
    let path = temp_archive_path("wrong-payload-kind", ".fse");
    let bytes = encode_archive_payload(FSEArchivePayloadKind::Index, &[]);

    fs::write(&path, bytes).unwrap();

    assert_eq!(
        read_row_mapped_archive_snapshot_file(&path),
        Err(FSERowMappedArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedPayloadKind {
                expected: FSEArchivePayloadKind::RowMappedIndex,
                actual: FSEArchivePayloadKind::Index
            }
        ))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn row_mapped_archive_file_reports_payload_checksum_mismatch() {
    let path = temp_archive_path("checksum-mismatch", ".fse");
    let bytes = corrupted_archive_payload(FSEArchivePayloadKind::RowMappedIndex);

    fs::write(&path, bytes).unwrap();
    let error = read_row_mapped_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSERowMappedArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::PayloadChecksumMismatch { .. }
        )
    ));

    let _ = fs::remove_file(path);
}
