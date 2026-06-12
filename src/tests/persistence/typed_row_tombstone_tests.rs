use crate::data::RowId;
use crate::persistence::{
    FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveSnapshotError,
};

#[test]
fn typed_row_tombstone_archive_snapshot_captures_deleted_row_ids() {
    let snapshot = FSETypedRowTombstoneArchiveSnapshot::from_row_ids(vec![
        RowId::new(100),
        RowId::new(104),
        RowId::new(109),
    ])
    .unwrap();

    assert_eq!(snapshot.deleted_row_ids, vec![100, 104, 109]);
    assert_eq!(snapshot.len(), 3);
    assert!(snapshot.contains(RowId::new(104)));
    assert!(!snapshot.contains(RowId::new(105)));
}

#[test]
fn typed_row_tombstone_archive_snapshot_allows_empty_marker_set() {
    let snapshot = FSETypedRowTombstoneArchiveSnapshot::default();

    assert!(snapshot.validate().is_ok());
    assert!(snapshot.is_empty());
    assert_eq!(snapshot.to_row_ids().unwrap(), Vec::<RowId>::new());
}

#[test]
fn typed_row_tombstone_archive_snapshot_rebuilds_row_ids() {
    let snapshot = FSETypedRowTombstoneArchiveSnapshot {
        deleted_row_ids: vec![7, 3, 11],
    };

    assert_eq!(
        snapshot.to_row_ids().unwrap(),
        vec![RowId::new(7), RowId::new(3), RowId::new(11)]
    );
}

#[test]
fn typed_row_tombstone_archive_snapshot_rejects_duplicate_row_ids() {
    let snapshot = FSETypedRowTombstoneArchiveSnapshot {
        deleted_row_ids: vec![7, 3, 7],
    };

    assert_eq!(
        snapshot.validate(),
        Err(FSETypedRowTombstoneArchiveSnapshotError::DuplicateRowId { row_id: 7 })
    );
    assert_eq!(
        FSETypedRowTombstoneArchiveSnapshot::from_row_ids(vec![RowId::new(7), RowId::new(7)]),
        Err(FSETypedRowTombstoneArchiveSnapshotError::DuplicateRowId { row_id: 7 })
    );
}
