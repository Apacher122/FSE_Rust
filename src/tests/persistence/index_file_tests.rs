use std::fs;
use std::io;
use std::path::PathBuf;

use crate::persistence::{
    FSEArchiveFileError, FSEArchiveFileOperation, FSEArchivePayloadHeaderError,
    FSEArchivePayloadKind, FSEArchiveSections, FSEIndexArchiveError, FSEIndexArchiveSnapshot,
    encode_archive_payload, load_index_archive_file, read_archive_snapshot_file,
    save_index_archive_file, save_index_archive_file_with_sections,
};
use crate::query::execute_query;
use crate::tests::support::{small_benchmark_fixture, sort_points};

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-index-archive-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn index_archive_file_saves_and_loads_query_equivalent_index() {
    let fixture = small_benchmark_fixture();
    let path = temp_archive_path("round-trip", ".fse");

    save_index_archive_file(&path, &fixture.index).unwrap();
    let loaded_index = load_index_archive_file(&path).unwrap();

    for workload in &fixture.workloads {
        let mut expected = execute_query(&fixture.index, &workload.query);
        let mut actual = execute_query(&loaded_index, &workload.query);

        sort_points(&mut expected);
        sort_points(&mut actual);

        assert_eq!(
            actual, expected,
            "loaded archive changed query results for workload `{}`",
            workload.name
        );
    }

    let _ = fs::remove_file(path);
}

#[test]
fn index_archive_file_records_explicit_section_metadata() {
    let fixture = small_benchmark_fixture();
    let path = temp_archive_path("typed-sections", ".fse");

    save_index_archive_file_with_sections(&path, &fixture.index, FSEArchiveSections::typed())
        .unwrap();
    let snapshot = read_archive_snapshot_file(&path).unwrap();

    assert_eq!(snapshot.manifest.sections, FSEArchiveSections::typed());

    let _ = fs::remove_file(path);
}

#[test]
fn index_archive_file_wraps_snapshot_bytes_with_index_payload_metadata() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let path = temp_archive_path("snapshot-bytes", ".fse");

    save_index_archive_file(&path, &fixture.index).unwrap();
    let saved_bytes = fs::read(&path).unwrap();
    let expected_payload = snapshot.to_archive_bytes().unwrap();
    let expected_bytes = encode_archive_payload(FSEArchivePayloadKind::Index, &expected_payload);

    assert_eq!(saved_bytes, expected_bytes);

    let _ = fs::remove_file(path);
}

#[test]
fn index_archive_file_reports_invalid_extension_on_save() {
    let fixture = small_benchmark_fixture();
    let path = temp_archive_path("wrong-save-extension", ".bin");

    assert_eq!(
        save_index_archive_file(&path, &fixture.index),
        Err(FSEIndexArchiveError::File(
            FSEArchiveFileError::InvalidFileExtension { path }
        ))
    );
}

#[test]
fn index_archive_file_reports_missing_file_on_load() {
    let path = temp_archive_path("missing-load", ".fse");

    assert_eq!(
        load_index_archive_file(&path),
        Err(FSEIndexArchiveError::File(FSEArchiveFileError::Io {
            operation: FSEArchiveFileOperation::Read,
            path,
            kind: io::ErrorKind::NotFound
        }))
    );
}

#[test]
fn index_archive_file_reports_payload_header_errors_on_load() {
    let path = temp_archive_path("invalid-load", ".fse");

    fs::write(&path, [0_u8; 4]).unwrap();
    let error = load_index_archive_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSEIndexArchiveError::File(FSEArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedEndOfArchive { .. }
        ))
    ));

    let _ = fs::remove_file(path);
}
