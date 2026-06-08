use crate::persistence::{
    FSEArchiveCodecError, FSEArchiveSnapshotError, FSEIndexArchiveSnapshot,
    decode_archive_snapshot, encode_archive_snapshot,
};
use crate::query::execute_query;
use crate::tests::support::{small_benchmark_fixture, sort_points};

#[test]
fn archive_snapshot_codec_round_trips_snapshot() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let bytes = encode_archive_snapshot(&snapshot).unwrap();
    let decoded = decode_archive_snapshot(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn archive_snapshot_codec_methods_round_trip_snapshot() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSEIndexArchiveSnapshot::from_archive_bytes(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn archive_snapshot_codec_preserves_query_results_after_decode() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let decoded = decode_archive_snapshot(&encode_archive_snapshot(&snapshot).unwrap()).unwrap();
    let rebuilt_index = decoded.to_index().unwrap();

    for workload in &fixture.workloads {
        let mut expected = execute_query(&fixture.index, &workload.query);
        let mut actual = execute_query(&rebuilt_index, &workload.query);

        sort_points(&mut expected);
        sort_points(&mut actual);

        assert_eq!(
            actual, expected,
            "decoded archive snapshot changed query results for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn archive_snapshot_codec_rejects_truncated_bytes() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let mut bytes = encode_archive_snapshot(&snapshot).unwrap();
    bytes.pop();

    assert!(matches!(
        decode_archive_snapshot(&bytes),
        Err(FSEArchiveCodecError::UnexpectedEndOfArchive { .. })
    ));
}

#[test]
fn archive_snapshot_codec_rejects_trailing_bytes() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let mut bytes = encode_archive_snapshot(&snapshot).unwrap();
    bytes.push(0);

    assert_eq!(
        decode_archive_snapshot(&bytes),
        Err(FSEArchiveCodecError::TrailingBytes { remaining: 1 })
    );
}

#[test]
fn archive_snapshot_codec_rejects_invalid_boolean() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let mut bytes = encode_archive_snapshot(&snapshot).unwrap();
    let last_byte = bytes.len() - 1;
    bytes[last_byte] = 2;

    assert!(matches!(
        decode_archive_snapshot(&bytes),
        Err(FSEArchiveCodecError::InvalidBoolean {
            field: "node.is_leaf",
            value: 2
        })
    ));
}

#[test]
fn archive_snapshot_codec_rejects_invalid_manifest_payload() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let mut bytes = encode_archive_snapshot(&snapshot).unwrap();

    bytes[8] = b'X';

    assert!(matches!(
        decode_archive_snapshot(&bytes),
        Err(FSEArchiveCodecError::Snapshot(
            FSEArchiveSnapshotError::Manifest(_)
        ))
    ));
}
