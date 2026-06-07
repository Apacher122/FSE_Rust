use crate::math::BoundingBoxError;
use crate::persistence::{
    FSEArchiveManifest, FSEArchiveSections, FSEArchiveSnapshotError, FSEIndexArchiveSnapshot,
};
use crate::query::execute_query;
use crate::tests::support::{small_benchmark_fixture, sort_points};

#[test]
fn archive_snapshot_captures_index_manifest_and_nodes() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();

    assert_eq!(
        snapshot.manifest.dimensions,
        fixture.index.dimensions as u32
    );
    assert_eq!(
        snapshot.manifest.record_count,
        fixture.index.root_node().cardinality as u64
    );
    assert_eq!(
        snapshot.manifest.node_count,
        fixture.index.node_count() as u64
    );
    assert_eq!(snapshot.manifest.root_node_id, fixture.index.root as u64);
    assert_eq!(snapshot.manifest.sections, FSEArchiveSections::empty());
    assert_eq!(snapshot.nodes.len(), fixture.index.node_count());

    for (position, node) in snapshot.nodes.iter().enumerate() {
        assert_eq!(node.id, position as u64);
    }
}

#[test]
fn archive_snapshot_accepts_typed_section_metadata() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index_with_sections(
        &fixture.index,
        FSEArchiveSections::typed(),
    )
    .unwrap();

    assert_eq!(snapshot.manifest.sections, FSEArchiveSections::typed());
    assert!(snapshot.validate().is_ok());
}

#[test]
fn archive_snapshot_rebuilds_query_equivalent_index() {
    let fixture = small_benchmark_fixture();
    let snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let rebuilt_index = snapshot.to_index().unwrap();

    for workload in &fixture.workloads {
        let mut expected = execute_query(&fixture.index, &workload.query);
        let mut actual = execute_query(&rebuilt_index, &workload.query);

        sort_points(&mut expected);
        sort_points(&mut actual);

        assert_eq!(
            actual, expected,
            "archive snapshot changed query results for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn archive_snapshot_reports_node_count_mismatch() {
    let fixture = small_benchmark_fixture();
    let mut snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    snapshot.manifest.node_count += 1;

    assert_eq!(
        snapshot.validate(),
        Err(FSEArchiveSnapshotError::NodeCountMismatch {
            manifest_node_count: fixture.index.node_count() as u64 + 1,
            actual_node_count: fixture.index.node_count() as u64
        })
    );
}

#[test]
fn archive_snapshot_reports_root_record_count_mismatch() {
    let fixture = small_benchmark_fixture();
    let mut snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    snapshot.manifest.record_count += 1;

    assert_eq!(
        snapshot.validate(),
        Err(FSEArchiveSnapshotError::RootRecordCountMismatch {
            root_node_id: snapshot.manifest.root_node_id,
            root_cardinality: fixture.index.root_node().cardinality as u64,
            manifest_record_count: fixture.index.root_node().cardinality as u64 + 1
        })
    );
}

#[test]
fn archive_snapshot_reports_node_id_mismatch() {
    let fixture = small_benchmark_fixture();
    let mut snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    snapshot.nodes[0].id = 1;

    assert_eq!(
        snapshot.validate(),
        Err(FSEArchiveSnapshotError::NodeIdMismatch {
            position: 0,
            node_id: 1
        })
    );
}

#[test]
fn archive_snapshot_reports_invalid_bounds() {
    let fixture = small_benchmark_fixture();
    let mut snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    snapshot.nodes[0].bounds_min[0] = snapshot.nodes[0].bounds_max[0] + 1.0;

    assert_eq!(
        snapshot.validate(),
        Err(FSEArchiveSnapshotError::Bounds {
            node_id: 0,
            source: BoundingBoxError::InvertedRange { dimension: 0 }
        })
    );
}

#[test]
fn archive_snapshot_reports_leaf_cardinality_mismatch() {
    let fixture = small_benchmark_fixture();
    let mut snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let leaf_position = snapshot
        .nodes
        .iter()
        .position(|node| node.is_leaf)
        .expect("fixture should include a leaf");
    let leaf_id = snapshot.nodes[leaf_position].id;
    let stored_rows = snapshot.nodes[leaf_position].residual_dimensions[0].len() as u64;
    snapshot.nodes[leaf_position].cardinality = stored_rows + 1;

    assert_eq!(
        snapshot.validate(),
        Err(FSEArchiveSnapshotError::LeafCardinalityMismatch {
            node_id: leaf_id,
            stored_rows,
            cardinality: stored_rows + 1
        })
    );
}

#[test]
fn archive_snapshot_reports_child_reference_out_of_range() {
    let fixture = small_benchmark_fixture();
    let mut snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    let internal_position = snapshot
        .nodes
        .iter()
        .position(|node| !node.is_leaf)
        .expect("fixture should include an internal node");
    let node_id = snapshot.nodes[internal_position].id;
    snapshot.nodes[internal_position]
        .children
        .push(snapshot.manifest.node_count);

    assert_eq!(
        snapshot.validate(),
        Err(FSEArchiveSnapshotError::ChildReferenceOutOfRange {
            node_id,
            child_id: snapshot.manifest.node_count,
            node_count: snapshot.manifest.node_count
        })
    );
}

#[test]
fn archive_snapshot_reports_manifest_errors() {
    let fixture = small_benchmark_fixture();
    let mut snapshot = FSEIndexArchiveSnapshot::from_index(&fixture.index).unwrap();
    snapshot.manifest = FSEArchiveManifest::new(2, 60, 23, 0, FSEArchiveSections::empty());
    snapshot.manifest.magic = "OTHER".to_string();

    assert!(matches!(
        snapshot.validate(),
        Err(FSEArchiveSnapshotError::Manifest(_))
    ));
}
