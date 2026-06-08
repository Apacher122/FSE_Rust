use std::fs;
use std::io;
use std::path::PathBuf;

use crate::persistence::{
    FSEArchiveCodecError, FSEArchiveFileError, FSEArchiveFileOperation, FSEIndexArchiveSnapshot,
    read_archive_snapshot_file, write_archive_snapshot_file,
};
use crate::query::execute_query;
use crate::tests::support::{small_benchmark_fixture, sort_points};

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-archive-file-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn archive_file_round_trips_snapshot_through_fse_file() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let path = temp_archive_path("round-trip", ".fse");

    write_archive_snapshot_file(&path, &snapshot).unwrap();
    let decoded = read_archive_snapshot_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn archive_file_methods_round_trip_snapshot() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let path = temp_archive_path("methods", ".fse");

    snapshot.write_to_archive_file(&path).unwrap();
    let decoded = FSEIndexArchiveSnapshot::read_from_archive_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn archive_file_preserves_query_results_after_read() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let path = temp_archive_path("query-results", ".fse");

    write_archive_snapshot_file(&path, &snapshot).unwrap();
    let decoded = read_archive_snapshot_file(&path).unwrap();
    let rebuilt_index = decoded.to_index().unwrap();

    for workload in &fixture.workloads {
        let mut expected = execute_query(&fixture.index, &workload.query);
        let mut actual = execute_query(&rebuilt_index, &workload.query);

        sort_points(&mut expected);
        sort_points(&mut actual);

        assert_eq!(
            actual, expected,
            "archive file changed query results for workload `{}`",
            workload.name
        );
    }

    let _ = fs::remove_file(path);
}

#[test]
fn archive_file_rejects_non_fse_extension_on_write() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let path = temp_archive_path("wrong-write-extension", ".bin");

    assert_eq!(
        write_archive_snapshot_file(&path, &snapshot),
        Err(FSEArchiveFileError::InvalidFileExtension { path })
    );
}

#[test]
fn archive_file_rejects_non_fse_extension_on_read() {
    let path = temp_archive_path("wrong-read-extension", ".bin");

    assert_eq!(
        read_archive_snapshot_file(&path),
        Err(FSEArchiveFileError::InvalidFileExtension { path })
    );
}

#[test]
fn archive_file_reports_missing_file_read_error() {
    let path = temp_archive_path("missing-read", ".fse");

    assert_eq!(
        read_archive_snapshot_file(&path),
        Err(FSEArchiveFileError::Io {
            operation: FSEArchiveFileOperation::Read,
            path,
            kind: io::ErrorKind::NotFound
        })
    );
}

#[test]
fn archive_file_reports_codec_errors_for_invalid_payload() {
    let path = temp_archive_path("invalid-payload", ".fse");

    fs::write(&path, [0_u8; 4]).unwrap();
    let error = read_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSEArchiveFileError::Codec(FSEArchiveCodecError::UnexpectedEndOfArchive { .. })
    ));

    let _ = fs::remove_file(path);
}
