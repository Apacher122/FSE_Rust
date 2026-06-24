use crate::math::Scalar;
use crate::persistence::{
    FSEArchiveCodecError, FSEArchiveManifest, FSEArchiveSections, FSEArchiveSnapshotError,
    FSEIndexArchiveSnapshot, FSEPartitionNodeArchiveRecord, decode_archive_snapshot,
    encode_archive_snapshot,
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
fn archive_snapshot_codec_uses_versioned_compact_container() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let bytes = encode_archive_snapshot(&snapshot).unwrap();

    assert_eq!(&bytes[..8], b"FSEIDX02");
}

#[test]
fn archive_snapshot_codec_decodes_legacy_snapshot_bytes() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let bytes = encode_legacy_archive_snapshot_for_test(&snapshot);
    let decoded = decode_archive_snapshot(&bytes).unwrap();

    assert!(!bytes.starts_with(b"FSEIDX02"));
    assert_eq!(decoded, snapshot);
}

#[test]
fn archive_snapshot_codec_compacts_empty_and_repeated_vectors() {
    let snapshot = repeated_archive_snapshot_for_test();
    let compact_bytes = encode_archive_snapshot(&snapshot).unwrap();
    let legacy_bytes = encode_legacy_archive_snapshot_for_test(&snapshot);
    let decoded = decode_archive_snapshot(&compact_bytes).unwrap();

    assert!(compact_bytes.len() < legacy_bytes.len());
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

    bytes[16] = b'X';

    assert!(matches!(
        decode_archive_snapshot(&bytes),
        Err(FSEArchiveCodecError::Snapshot(
            FSEArchiveSnapshotError::Manifest(_)
        ))
    ));
}

fn repeated_archive_snapshot_for_test() -> FSEIndexArchiveSnapshot {
    let snapshot = FSEIndexArchiveSnapshot {
        manifest: FSEArchiveManifest::try_new(2, 2, 1, 0, FSEArchiveSections::empty()).unwrap(),
        nodes: vec![FSEPartitionNodeArchiveRecord {
            id: 0,
            centroid: vec![10.0, 10.0],
            bounds_min: vec![8.0, 8.0],
            bounds_max: vec![12.0, 12.0],
            residual_dimensions: vec![vec![0.0, 0.0], vec![0.0, 0.0]],
            cardinality: 2,
            children: Vec::new(),
            is_leaf: true,
        }],
    };

    snapshot.validate().unwrap();
    snapshot
}

fn encode_legacy_archive_snapshot_for_test(snapshot: &FSEIndexArchiveSnapshot) -> Vec<u8> {
    let mut bytes = Vec::new();

    write_legacy_manifest_for_test(&mut bytes, &snapshot.manifest);

    for node in &snapshot.nodes {
        write_legacy_node_for_test(&mut bytes, node);
    }

    bytes
}

fn write_legacy_manifest_for_test(bytes: &mut Vec<u8>, manifest: &FSEArchiveManifest) {
    write_legacy_string_for_test(bytes, &manifest.magic);
    write_legacy_u32_for_test(bytes, manifest.format_version);
    write_legacy_string_for_test(bytes, &manifest.file_extension);
    write_legacy_u32_for_test(bytes, manifest.scalar_size_bytes);
    write_legacy_u32_for_test(bytes, manifest.dimensions);
    write_legacy_u64_for_test(bytes, manifest.record_count);
    write_legacy_u64_for_test(bytes, manifest.node_count);
    write_legacy_u64_for_test(bytes, manifest.root_node_id);
    write_legacy_bool_for_test(bytes, manifest.sections.dataset_metadata);
    write_legacy_bool_for_test(bytes, manifest.sections.schema_metadata);
    write_legacy_bool_for_test(bytes, manifest.sections.encoder_metadata);
}

fn write_legacy_node_for_test(bytes: &mut Vec<u8>, node: &FSEPartitionNodeArchiveRecord) {
    write_legacy_u64_for_test(bytes, node.id);
    write_legacy_scalar_vec_for_test(bytes, &node.centroid);
    write_legacy_scalar_vec_for_test(bytes, &node.bounds_min);
    write_legacy_scalar_vec_for_test(bytes, &node.bounds_max);
    write_legacy_u64_for_test(bytes, node.residual_dimensions.len() as u64);

    for dimension in &node.residual_dimensions {
        write_legacy_scalar_vec_for_test(bytes, dimension);
    }

    write_legacy_u64_for_test(bytes, node.cardinality);
    write_legacy_u64_vec_for_test(bytes, &node.children);
    write_legacy_bool_for_test(bytes, node.is_leaf);
}

fn write_legacy_string_for_test(bytes: &mut Vec<u8>, value: &str) {
    write_legacy_u64_for_test(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn write_legacy_scalar_vec_for_test(bytes: &mut Vec<u8>, values: &[Scalar]) {
    write_legacy_u64_for_test(bytes, values.len() as u64);

    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn write_legacy_u64_vec_for_test(bytes: &mut Vec<u8>, values: &[u64]) {
    write_legacy_u64_for_test(bytes, values.len() as u64);

    for value in values {
        write_legacy_u64_for_test(bytes, *value);
    }
}

fn write_legacy_bool_for_test(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}

fn write_legacy_u32_for_test(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_legacy_u64_for_test(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
