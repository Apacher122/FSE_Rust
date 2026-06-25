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
    let v3_bytes = encode_v3_archive_snapshot_for_test(&snapshot);
    let v2_bytes = encode_v2_archive_snapshot_for_test(&snapshot);

    assert_eq!(&bytes[..8], b"FSEIDX04");
    assert!(bytes.len() <= v3_bytes.len());
    assert!(bytes.len() < v2_bytes.len());
}

#[test]
fn archive_snapshot_codec_decodes_v3_fixed_shape_snapshot_bytes() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let bytes = encode_v3_archive_snapshot_for_test(&snapshot);
    let decoded = decode_archive_snapshot(&bytes).unwrap();

    assert!(bytes.starts_with(b"FSEIDX03"));
    assert_eq!(decoded, snapshot);
}

#[test]
fn archive_snapshot_codec_decodes_v2_compact_snapshot_bytes() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let bytes = encode_v2_archive_snapshot_for_test(&snapshot);
    let decoded = decode_archive_snapshot(&bytes).unwrap();

    assert!(bytes.starts_with(b"FSEIDX02"));
    assert_eq!(decoded, snapshot);
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
fn archive_snapshot_codec_byte_plane_scalars_reduce_fixed_shape_bytes() {
    let snapshot = byte_plane_archive_snapshot_for_test();
    let byte_plane_bytes = encode_archive_snapshot(&snapshot).unwrap();
    let fixed_shape_bytes = encode_v3_archive_snapshot_for_test(&snapshot);
    let decoded = decode_archive_snapshot(&byte_plane_bytes).unwrap();

    assert!(byte_plane_bytes.len() < fixed_shape_bytes.len());
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

fn byte_plane_archive_snapshot_for_test() -> FSEIndexArchiveSnapshot {
    let snapshot = FSEIndexArchiveSnapshot {
        manifest: FSEArchiveManifest::try_new(4, 4, 1, 0, FSEArchiveSections::empty()).unwrap(),
        nodes: vec![FSEPartitionNodeArchiveRecord {
            id: 0,
            centroid: vec![1.0, 2.0, 3.0, 4.0],
            bounds_min: vec![0.0, 0.0, 0.0, 0.0],
            bounds_max: vec![8.0, 8.0, 8.0, 8.0],
            residual_dimensions: vec![
                vec![1.0, 2.0, 3.0, 4.0],
                vec![2.0, 3.0, 4.0, 5.0],
                vec![3.0, 4.0, 5.0, 6.0],
                vec![4.0, 5.0, 6.0, 7.0],
            ],
            cardinality: 4,
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

fn encode_v3_archive_snapshot_for_test(snapshot: &FSEIndexArchiveSnapshot) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"FSEIDX03");
    write_legacy_manifest_for_test(&mut bytes, &snapshot.manifest);

    for node in &snapshot.nodes {
        write_v3_node_for_test(&mut bytes, node, snapshot.manifest.dimensions as usize);
    }

    bytes
}

fn encode_v2_archive_snapshot_for_test(snapshot: &FSEIndexArchiveSnapshot) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"FSEIDX02");
    write_legacy_manifest_for_test(&mut bytes, &snapshot.manifest);

    for node in &snapshot.nodes {
        write_v2_node_for_test(&mut bytes, node);
    }

    bytes
}

fn write_v3_node_for_test(
    bytes: &mut Vec<u8>,
    node: &FSEPartitionNodeArchiveRecord,
    dimensions: usize,
) {
    write_var_u64_for_test(bytes, node.id);
    write_fixed_compact_scalar_vec_for_test(bytes, &node.centroid, dimensions);
    write_fixed_compact_scalar_vec_for_test(bytes, &node.bounds_min, dimensions);
    write_fixed_compact_scalar_vec_for_test(bytes, &node.bounds_max, dimensions);

    let residual_rows = node.residual_dimensions.first().map_or(0, Vec::len);
    write_var_u64_for_test(bytes, residual_rows as u64);

    if residual_rows > 0 {
        for dimension in &node.residual_dimensions {
            write_fixed_compact_scalar_vec_for_test(bytes, dimension, residual_rows);
        }
    }

    write_var_u64_for_test(bytes, node.cardinality);
    write_compact_u64_vec_for_test(bytes, &node.children);
    write_legacy_bool_for_test(bytes, node.is_leaf);
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

fn write_fixed_compact_scalar_vec_for_test(
    bytes: &mut Vec<u8>,
    values: &[Scalar],
    expected_len: usize,
) {
    if expected_len == 0 {
        bytes.push(1);
        return;
    }

    if let Some(value) = repeated_scalar_for_test(values) {
        bytes.push(2);
        bytes.extend_from_slice(&value.to_le_bytes());
        return;
    }

    bytes.push(0);

    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn write_v2_node_for_test(bytes: &mut Vec<u8>, node: &FSEPartitionNodeArchiveRecord) {
    write_legacy_u64_for_test(bytes, node.id);
    write_compact_scalar_vec_for_test(bytes, &node.centroid);
    write_compact_scalar_vec_for_test(bytes, &node.bounds_min);
    write_compact_scalar_vec_for_test(bytes, &node.bounds_max);
    write_legacy_u64_for_test(bytes, node.residual_dimensions.len() as u64);

    for dimension in &node.residual_dimensions {
        write_compact_scalar_vec_for_test(bytes, dimension);
    }

    write_legacy_u64_for_test(bytes, node.cardinality);
    write_compact_u64_vec_for_test(bytes, &node.children);
    write_legacy_bool_for_test(bytes, node.is_leaf);
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

fn write_compact_scalar_vec_for_test(bytes: &mut Vec<u8>, values: &[Scalar]) {
    if values.is_empty() {
        bytes.push(1);
        return;
    }

    if let Some(value) = repeated_scalar_for_test(values) {
        bytes.push(2);
        write_legacy_u64_for_test(bytes, values.len() as u64);
        bytes.extend_from_slice(&value.to_le_bytes());
        return;
    }

    bytes.push(0);
    write_legacy_scalar_vec_for_test(bytes, values);
}

fn write_compact_u64_vec_for_test(bytes: &mut Vec<u8>, values: &[u64]) {
    if values.is_empty() {
        bytes.push(2);
        return;
    }

    let raw_len = std::mem::size_of::<u64>() * values.len();
    let varint_len = values
        .iter()
        .map(|value| var_u64_len(*value))
        .sum::<usize>();

    if varint_len < raw_len {
        bytes.push(1);
        write_legacy_u64_for_test(bytes, values.len() as u64);

        for value in values {
            write_var_u64_for_test(bytes, *value);
        }
    } else {
        bytes.push(0);
        write_legacy_u64_vec_for_test(bytes, values);
    }
}

fn repeated_scalar_for_test(values: &[Scalar]) -> Option<Scalar> {
    if values.len() < 2 {
        return None;
    }

    let value = values[0];

    values
        .iter()
        .all(|other| other.to_bits() == value.to_bits())
        .then_some(value)
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

fn write_var_u64_for_test(bytes: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;

        if value != 0 {
            byte |= 0x80;
        }

        bytes.push(byte);

        if value == 0 {
            break;
        }
    }
}

fn var_u64_len(mut value: u64) -> usize {
    let mut len = 1;

    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }

    len
}
