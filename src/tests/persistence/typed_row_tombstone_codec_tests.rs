use crate::data::RowId;
use crate::persistence::{
    FSETypedRowTombstoneArchiveCodecError, FSETypedRowTombstoneArchiveSnapshot,
    FSETypedRowTombstoneArchiveSnapshotError, decode_typed_row_tombstone_archive_snapshot,
    encode_typed_row_tombstone_archive_snapshot,
};

#[test]
fn typed_row_tombstone_archive_codec_round_trips_snapshot() {
    let snapshot = sample_snapshot();
    let bytes = encode_typed_row_tombstone_archive_snapshot(&snapshot).unwrap();
    let decoded = decode_typed_row_tombstone_archive_snapshot(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_row_tombstone_archive_codec_methods_round_trip_snapshot() {
    let snapshot = sample_snapshot();
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSETypedRowTombstoneArchiveSnapshot::from_archive_bytes(&bytes).unwrap();

    assert_eq!(decoded, snapshot);
}

#[test]
fn typed_row_tombstone_archive_codec_rebuilds_row_ids_after_decode() {
    let snapshot = sample_snapshot();
    let bytes = snapshot.to_archive_bytes().unwrap();
    let decoded = FSETypedRowTombstoneArchiveSnapshot::from_archive_bytes(&bytes).unwrap();

    assert_eq!(
        decoded.to_row_ids().unwrap(),
        vec![RowId::new(100), RowId::new(104), RowId::new(109)]
    );
}

#[test]
fn typed_row_tombstone_archive_codec_rejects_invalid_snapshot_on_encode() {
    let snapshot = FSETypedRowTombstoneArchiveSnapshot {
        deleted_row_ids: vec![100, 100],
    };

    assert_eq!(
        encode_typed_row_tombstone_archive_snapshot(&snapshot),
        Err(FSETypedRowTombstoneArchiveCodecError::Snapshot(
            FSETypedRowTombstoneArchiveSnapshotError::DuplicateRowId { row_id: 100 },
        ))
    );
}

#[test]
fn typed_row_tombstone_archive_codec_rejects_duplicate_row_ids_after_decode() {
    let bytes = tombstone_bytes(&[100, 100]);

    assert_eq!(
        decode_typed_row_tombstone_archive_snapshot(&bytes),
        Err(FSETypedRowTombstoneArchiveCodecError::Snapshot(
            FSETypedRowTombstoneArchiveSnapshotError::DuplicateRowId { row_id: 100 },
        ))
    );
}

#[test]
fn typed_row_tombstone_archive_codec_rejects_truncated_bytes() {
    let snapshot = sample_snapshot();
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    bytes.pop();

    assert!(matches!(
        decode_typed_row_tombstone_archive_snapshot(&bytes),
        Err(FSETypedRowTombstoneArchiveCodecError::UnexpectedEndOfArchive { .. })
    ));
}

#[test]
fn typed_row_tombstone_archive_codec_rejects_trailing_bytes() {
    let snapshot = sample_snapshot();
    let mut bytes = snapshot.to_archive_bytes().unwrap();
    bytes.push(99);

    assert_eq!(
        decode_typed_row_tombstone_archive_snapshot(&bytes),
        Err(FSETypedRowTombstoneArchiveCodecError::TrailingBytes { remaining: 1 })
    );
}

fn sample_snapshot() -> FSETypedRowTombstoneArchiveSnapshot {
    FSETypedRowTombstoneArchiveSnapshot::from_row_ids(vec![
        RowId::new(100),
        RowId::new(104),
        RowId::new(109),
    ])
    .unwrap()
}

fn tombstone_bytes(row_ids: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(row_ids.len() as u64).to_le_bytes());

    for row_id in row_ids {
        bytes.extend_from_slice(&row_id.to_le_bytes());
    }

    bytes
}
