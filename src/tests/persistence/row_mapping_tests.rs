use crate::build::{BuildConfig, FSEBuilder};
use crate::data::RowId;
use crate::encoding::EncodedRecordBatch;
use crate::math::Vector;
use crate::persistence::{
    FSELeafRowIdArchiveRecord, FSERowMappedArchiveSnapshotError, FSERowMappedIndexArchiveSnapshot,
};

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

#[test]
fn row_mapped_archive_snapshot_captures_leaf_row_ids() {
    let mapped = row_mapped_fixture();
    let snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();

    assert_eq!(snapshot.index.manifest.node_count, 3);
    assert_eq!(
        snapshot.leaf_row_id_records,
        vec![
            FSELeafRowIdArchiveRecord {
                node_id: 1,
                row_ids: vec![10, 11],
            },
            FSELeafRowIdArchiveRecord {
                node_id: 2,
                row_ids: vec![12, 13],
            },
        ]
    );
}

#[test]
fn row_mapped_archive_snapshot_rebuilds_row_mapping() {
    let mapped = row_mapped_fixture();
    let snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    let rebuilt = snapshot.to_row_mapped_index().unwrap();

    assert_eq!(rebuilt.index(), mapped.index());
    assert_eq!(
        rebuilt.leaf_row_ids(1),
        Some([RowId::new(10), RowId::new(11)].as_slice())
    );
    assert_eq!(
        rebuilt.leaf_row_ids(2),
        Some([RowId::new(12), RowId::new(13)].as_slice())
    );
    assert_eq!(rebuilt.leaf_row_ids(0), None);
    assert_eq!(rebuilt.row_id_for_leaf_row(2, 1), Some(RowId::new(13)));
}

#[test]
fn row_mapped_archive_snapshot_reports_unknown_node_mapping() {
    let mapped = row_mapped_fixture();
    let mut snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    snapshot
        .leaf_row_id_records
        .push(FSELeafRowIdArchiveRecord {
            node_id: 3,
            row_ids: Vec::new(),
        });

    assert_eq!(
        snapshot.validate(),
        Err(FSERowMappedArchiveSnapshotError::UnknownNode {
            node_id: 3,
            node_count: 3
        })
    );
}

#[test]
fn row_mapped_archive_snapshot_reports_internal_node_mapping() {
    let mapped = row_mapped_fixture();
    let mut snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    snapshot
        .leaf_row_id_records
        .push(FSELeafRowIdArchiveRecord {
            node_id: 0,
            row_ids: Vec::new(),
        });

    assert_eq!(
        snapshot.validate(),
        Err(FSERowMappedArchiveSnapshotError::InternalNodeMapping { node_id: 0 })
    );
}

#[test]
fn row_mapped_archive_snapshot_reports_duplicate_leaf_mapping() {
    let mapped = row_mapped_fixture();
    let mut snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    snapshot
        .leaf_row_id_records
        .push(FSELeafRowIdArchiveRecord {
            node_id: 1,
            row_ids: vec![20, 21],
        });

    assert_eq!(
        snapshot.validate(),
        Err(FSERowMappedArchiveSnapshotError::DuplicateLeafMapping { node_id: 1 })
    );
}

#[test]
fn row_mapped_archive_snapshot_reports_missing_leaf_mapping() {
    let mapped = row_mapped_fixture();
    let mut snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    snapshot
        .leaf_row_id_records
        .retain(|record| record.node_id != 2);

    assert_eq!(
        snapshot.validate(),
        Err(FSERowMappedArchiveSnapshotError::MissingLeafMapping { node_id: 2 })
    );
}

#[test]
fn row_mapped_archive_snapshot_reports_leaf_row_count_mismatch() {
    let mapped = row_mapped_fixture();
    let mut snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    snapshot.leaf_row_id_records[0].row_ids.pop();

    assert_eq!(
        snapshot.validate(),
        Err(FSERowMappedArchiveSnapshotError::LeafRowIdCountMismatch {
            node_id: 1,
            row_id_count: 1,
            leaf_row_count: 2
        })
    );
}

#[test]
fn row_mapped_archive_snapshot_reports_duplicate_row_id() {
    let mapped = row_mapped_fixture();
    let mut snapshot = FSERowMappedIndexArchiveSnapshot::from_row_mapped_index(&mapped).unwrap();
    snapshot.leaf_row_id_records[1].row_ids[0] = 10;

    assert_eq!(
        snapshot.validate(),
        Err(FSERowMappedArchiveSnapshotError::DuplicateRowId { row_id: 10 })
    );
}
