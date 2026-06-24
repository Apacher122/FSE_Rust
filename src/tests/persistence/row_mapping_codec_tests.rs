use crate::build::{BuildConfig, FSEBuilder};
use crate::data::RowId;
use crate::encoding::EncodedRecordBatch;
use crate::math::Vector;
use crate::persistence::{
    FSERowMappedArchiveCodecError, FSERowMappedArchiveSnapshotError,
    FSERowMappedIndexArchiveSnapshot, decode_row_mapped_archive_snapshot, encode_archive_snapshot,
    encode_row_mapped_archive_snapshot,
};

fn row_mapped_snapshot() -> FSERowMappedIndexArchiveSnapshot {
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
    let mapped = builder
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("valid encoded batch should build");

    FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped)
        .expect("valid row-mapped index should snapshot")
}

#[test]
fn row_mapped_archive_codec_round_trips_snapshot() {
    let snapshot = row_mapped_snapshot();
    let bytes = encode_row_mapped_archive_snapshot(&snapshot).unwrap();
    let decoded = decode_row_mapped_archive_snapshot(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn row_mapped_archive_codec_compacts_row_id_vectors() {
    let snapshot = row_mapped_snapshot();
    let compact_bytes = encode_row_mapped_archive_snapshot(&snapshot).unwrap();
    let legacy_bytes = encode_legacy_row_mapped_archive_snapshot(&snapshot);
    let decoded = decode_row_mapped_archive_snapshot(&compact_bytes).unwrap();

    assert!(compact_bytes.starts_with(b"FSERMV02"));
    assert!(compact_bytes.len() < legacy_bytes.len());
    assert_eq!(decoded, snapshot);
}

#[test]
fn row_mapped_archive_codec_decodes_legacy_row_id_vectors() {
    let snapshot = row_mapped_snapshot();
    let legacy_bytes = encode_legacy_row_mapped_archive_snapshot(&snapshot);
    let decoded = decode_row_mapped_archive_snapshot(&legacy_bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn row_mapped_archive_codec_methods_round_trip_snapshot() {
    let snapshot = row_mapped_snapshot();
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSERowMappedIndexArchiveSnapshot::from_archive_bytes(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn row_mapped_archive_codec_preserves_row_mapping_after_decode() {
    let snapshot = row_mapped_snapshot();
    let decoded =
        decode_row_mapped_archive_snapshot(&encode_row_mapped_archive_snapshot(&snapshot).unwrap())
            .unwrap();
    let rebuilt = decoded.to_row_mapped_index().unwrap();

    assert_eq!(
        rebuilt.leaf_row_ids(1),
        Some([RowId::new(10), RowId::new(11)].as_slice())
    );
    assert_eq!(
        rebuilt.leaf_row_ids(2),
        Some([RowId::new(12), RowId::new(13)].as_slice())
    );
    assert_eq!(rebuilt.row_id_for_leaf_row(2, 1), Some(RowId::new(13)));
}

#[test]
fn row_mapped_archive_codec_rejects_truncated_bytes() {
    let snapshot = row_mapped_snapshot();
    let mut bytes = encode_row_mapped_archive_snapshot(&snapshot).unwrap();
    bytes.pop();

    assert!(matches!(
        decode_row_mapped_archive_snapshot(&bytes),
        Err(FSERowMappedArchiveCodecError::UnexpectedEndOfArchive { .. })
    ));
}

#[test]
fn row_mapped_archive_codec_rejects_trailing_bytes() {
    let snapshot = row_mapped_snapshot();
    let mut bytes = encode_row_mapped_archive_snapshot(&snapshot).unwrap();
    bytes.push(0);

    assert_eq!(
        decode_row_mapped_archive_snapshot(&bytes),
        Err(FSERowMappedArchiveCodecError::TrailingBytes { remaining: 1 })
    );
}

#[test]
fn row_mapped_archive_codec_rejects_invalid_embedded_index_payload() {
    let snapshot = row_mapped_snapshot();
    let mut bytes = encode_row_mapped_archive_snapshot(&snapshot).unwrap();

    bytes[16] = b'X';

    assert!(matches!(
        decode_row_mapped_archive_snapshot(&bytes),
        Err(FSERowMappedArchiveCodecError::IndexCodec(_))
    ));
}

#[test]
fn row_mapped_archive_codec_rejects_invalid_row_mapping_payload() {
    let snapshot = row_mapped_snapshot();
    let mut bytes = encode_row_mapped_archive_snapshot(&snapshot).unwrap();
    let last_row_id_offset = bytes.len() - 1;
    bytes[last_row_id_offset] = 10;

    assert_eq!(
        decode_row_mapped_archive_snapshot(&bytes),
        Err(FSERowMappedArchiveCodecError::Snapshot(
            FSERowMappedArchiveSnapshotError::DuplicateRowId { row_id: 10 }
        ))
    );
}

fn encode_legacy_row_mapped_archive_snapshot(
    snapshot: &FSERowMappedIndexArchiveSnapshot,
) -> Vec<u8> {
    let index_bytes = encode_archive_snapshot(&snapshot.index).unwrap();
    let mut bytes = Vec::new();

    write_byte_vec(&mut bytes, &index_bytes);
    write_u64(&mut bytes, snapshot.leaf_row_id_records.len() as u64);

    for record in &snapshot.leaf_row_id_records {
        write_u64(&mut bytes, record.node_id);
        write_u64_vec(&mut bytes, &record.row_ids);
    }

    bytes
}

fn write_byte_vec(bytes: &mut Vec<u8>, values: &[u8]) {
    write_u64(bytes, values.len() as u64);
    bytes.extend_from_slice(values);
}

fn write_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    write_u64(bytes, values.len() as u64);

    for value in values {
        write_u64(bytes, *value);
    }
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
