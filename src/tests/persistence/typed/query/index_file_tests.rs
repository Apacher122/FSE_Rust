use std::fs;
use std::io;
use std::path::PathBuf;

use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSECsvImportOptions, FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch,
    FSERecordBatchError, FSESchema, FSESchemaDimensionMapping, FSEValue, RowId,
    record_batch_from_csv_file,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FSEEncodingError, FSEFieldEncoderMetadata,
    FSERecordBatchEncodingError, FSERecordEncoderMetadata, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::persistence::{
    FSEArchiveCompactionOperationMetadataError, FSEArchiveFileOperation,
    FSEArchiveMaintenanceAction, FSEArchiveMaintenanceError, FSEArchiveMaintenancePolicy,
    FSEArchiveMaintenanceReason, FSEArchivePayloadHeaderError, FSEArchivePayloadKind,
    FSEArchiveRebuildOperationMetadata, FSEArchiveRebuildReason,
    FSETypedQueryIndexArchiveCompactionError, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFileError, FSETypedQueryIndexArchiveMaintenanceError,
    FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexCompactionError,
    append_typed_query_index_archive_file, build_typed_query_index_archive_file,
    build_typed_query_index_archive_file_with_encoder_metadata,
    compact_typed_query_index_archive_file, encode_archive_payload,
    load_typed_query_index_archive_file, load_typed_query_index_archive_with_tombstones,
    load_typed_row_tombstone_archive_file, maintain_typed_query_index_archive_file,
    read_typed_query_index_archive_snapshot_file, save_typed_query_index_archive_file,
    save_typed_query_index_archive_file_with_encoder_metadata,
    save_typed_row_tombstone_archive_file, write_typed_query_index_archive_snapshot_file,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryIndexAppendError,
    TypedQueryIndexBuildError, TypedQueryPlanBuilder,
};

use super::super::corrupted_archive_payload;

#[test]
fn typed_query_index_archive_file_round_trips_snapshot_through_fse_file() {
    let query_index = typed_query_index();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    let path = temp_archive_path("snapshot-round-trip", ".fse");

    write_typed_query_index_archive_snapshot_file(&path, &snapshot).unwrap();
    let decoded = read_typed_query_index_archive_snapshot_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_methods_round_trip_snapshot() {
    let query_index = typed_query_index();
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(&query_index).unwrap();
    let path = temp_archive_path("snapshot-methods", ".fse");

    snapshot.write_to_archive_file(&path).unwrap();
    let decoded = FSETypedQueryIndexArchiveSnapshot::read_from_archive_file(&path).unwrap();

    assert_eq!(decoded, snapshot);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_saves_and_loads_query_equivalent_index() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let path = temp_archive_path("index-round-trip", ".fse");

    save_typed_query_index_archive_file(&path, &query_index).unwrap();
    let loaded = load_typed_query_index_archive_file(&path).unwrap();

    assert_eq!(loaded.batch(), query_index.batch());
    assert_eq!(loaded.index(), query_index.index());
    assert_eq!(
        loaded.query_row_ids(&plan).unwrap(),
        query_index.query_row_ids(&plan).unwrap()
    );
    assert_eq!(
        loaded.query_rows(&plan).unwrap(),
        query_index.query_rows(&plan).unwrap()
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_saves_record_encoder_metadata() {
    let query_index = typed_query_index_with_reverse_category_encoder();
    let metadata = reverse_category_encoder_metadata();
    let path = temp_archive_path("index-metadata-round-trip", ".fse");

    save_typed_query_index_archive_file_with_encoder_metadata(
        &path,
        &query_index,
        metadata.clone(),
    )
    .unwrap();
    let snapshot = read_typed_query_index_archive_snapshot_file(&path).unwrap();

    assert_eq!(snapshot.record_encoder, metadata);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_builds_index_and_writes_fse_file() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let plan = score_and_class_plan(&schema, &mapping);
    let path = temp_archive_path("build-index", ".fse");

    let query_index =
        build_typed_query_index_archive_file(&path, batch, &encoder, &builder).unwrap();
    let loaded = load_typed_query_index_archive_file(&path).unwrap();

    assert!(path.exists());
    assert_eq!(loaded, query_index);
    assert_eq!(
        loaded.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_builds_with_record_encoder_metadata() {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = reverse_category_encoder(&schema);
    let metadata = reverse_category_encoder_metadata();
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let path = temp_archive_path("build-index-with-metadata", ".fse");

    let query_index = build_typed_query_index_archive_file_with_encoder_metadata(
        &path,
        batch,
        &encoder,
        metadata.clone(),
        &builder,
    )
    .unwrap();
    let snapshot = read_typed_query_index_archive_snapshot_file(&path).unwrap();
    let loaded = load_typed_query_index_archive_file(&path).unwrap();

    assert_eq!(snapshot.record_encoder, metadata);
    assert_eq!(loaded, query_index);

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_builds_fse_file_from_csv_record_batch() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let csv_path = temp_csv_path("build-index-from-csv");
    let archive_path = temp_archive_path("build-index-from-csv", ".fse");
    let csv = "\
entity_id,score,class,observed_at
100,12.5,alpha,1000
101,12.5,beta,1100
102,25.0,alpha,1200
103,18.0,alpha,1300
";
    fs::write(&csv_path, csv).unwrap();

    let batch = record_batch_from_csv_file(
        &csv_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
    )
    .unwrap();
    let query_index =
        build_typed_query_index_archive_file(&archive_path, batch, &encoder, &builder).unwrap();
    let loaded = load_typed_query_index_archive_file(&archive_path).unwrap();
    let plan = score_and_class_plan(&schema, &mapping);

    assert_eq!(loaded, query_index);
    assert_eq!(
        loaded.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn typed_query_index_archive_file_build_reports_encoding_errors() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let batch = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100)],
        vec![entity_record(&schema, 1, 12.5, "gamma", 1_000)],
    );
    let path = temp_archive_path("build-encoding-error", ".fse");

    assert_eq!(
        build_typed_query_index_archive_file(&path, batch, &encoder, &builder),
        Err(FSETypedQueryIndexArchiveError::Build(
            TypedQueryIndexBuildError::Encoding(FSERecordBatchEncodingError::RecordEncoding {
                record: 0,
                row_id: RowId::new(100),
                source: FSEEncodingError::UnsupportedValue {
                    reason: "category 'gamma' is not in dictionary".to_string(),
                },
            })
        ))
    );
    assert!(!path.exists());
}

#[test]
fn typed_query_index_archive_file_build_reports_invalid_extension_before_building() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let batch = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100)],
        vec![entity_record(&schema, 1, 12.5, "gamma", 1_000)],
    );
    let path = temp_archive_path("build-invalid-extension", ".bin");

    assert_eq!(
        build_typed_query_index_archive_file(&path, batch, &encoder, &builder),
        Err(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::InvalidFileExtension { path }
        ))
    );
}

#[test]
fn typed_query_index_archive_file_appends_records_and_rebuilds_archive() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let appended = appended_entity_batch(&schema);
    let plan = score_and_class_plan(&schema, &mapping);
    let path = temp_archive_path("index-append", ".fse");

    save_typed_query_index_archive_file(&path, &query_index).unwrap();
    let result =
        append_typed_query_index_archive_file(&path, &appended, &encoder, &builder).unwrap();
    let loaded = load_typed_query_index_archive_file(&path).unwrap();
    let mut matches = loaded.query_row_ids(&plan).unwrap();
    matches.sort();

    assert_eq!(
        result.append_metadata.payload_kind,
        FSEArchivePayloadKind::TypedQueryIndex
    );
    assert_eq!(result.append_metadata.base_record_count, 4);
    assert_eq!(result.append_metadata.appended_record_count, 2);
    assert_eq!(result.append_metadata.resulting_record_count, 6);
    assert_eq!(result.rebuild_plan.reason, FSEArchiveRebuildReason::Append);
    assert_eq!(
        result.rebuild_plan.operation,
        FSEArchiveRebuildOperationMetadata::Append(result.append_metadata)
    );
    assert!(result.rebuild_plan.requires_full_archive_rebuild);
    assert_eq!(result.query_index, loaded);
    assert_eq!(loaded.batch().len(), 6);
    assert_eq!(
        matches,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_reports_duplicate_row_id_append() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let appended = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100)],
        vec![entity_record(&schema, 5, 16.0, "alpha", 1_400)],
    );
    let path = temp_archive_path("append-duplicate-row-id", ".fse");

    save_typed_query_index_archive_file(&path, &query_index).unwrap();

    assert_eq!(
        append_typed_query_index_archive_file(&path, &appended, &encoder, &builder),
        Err(FSETypedQueryIndexArchiveError::Append(
            TypedQueryIndexAppendError::RecordBatch(FSERecordBatchError::DuplicateRowId {
                row_id: RowId::new(100)
            })
        ))
    );
    assert_eq!(
        load_typed_query_index_archive_file(&path).unwrap(),
        query_index
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_reports_append_schema_mismatch() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let appended = mismatched_appended_batch();
    let path = temp_archive_path("append-schema-mismatch", ".fse");

    save_typed_query_index_archive_file(&path, &query_index).unwrap();

    assert_eq!(
        append_typed_query_index_archive_file(&path, &appended, &encoder, &builder),
        Err(FSETypedQueryIndexArchiveError::Append(
            TypedQueryIndexAppendError::RecordBatch(FSERecordBatchError::SchemaMismatch)
        ))
    );
    assert_eq!(
        load_typed_query_index_archive_file(&path).unwrap(),
        query_index
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_reports_empty_append_batch() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let appended = FSERecordBatch::new(schema, Vec::new(), Vec::new());
    let path = temp_archive_path("empty-append", ".fse");

    save_typed_query_index_archive_file(&path, &query_index).unwrap();

    assert_eq!(
        append_typed_query_index_archive_file(&path, &appended, &encoder, &builder),
        Err(FSETypedQueryIndexArchiveError::Append(
            TypedQueryIndexAppendError::RecordBatch(FSERecordBatchError::EmptyAppendBatch)
        ))
    );
    assert_eq!(
        load_typed_query_index_archive_file(&path).unwrap(),
        query_index
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_reports_rebuild_failure_during_append() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let appended = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(104)],
        vec![entity_record(&schema, 5, 16.0, "gamma", 1_400)],
    );
    let path = temp_archive_path("append-rebuild-failure", ".fse");

    save_typed_query_index_archive_file(&path, &query_index).unwrap();

    assert_eq!(
        append_typed_query_index_archive_file(&path, &appended, &encoder, &builder),
        Err(FSETypedQueryIndexArchiveError::Append(
            TypedQueryIndexAppendError::Rebuild(TypedQueryIndexBuildError::Encoding(
                FSERecordBatchEncodingError::RecordEncoding {
                    record: 4,
                    row_id: RowId::new(104),
                    source: FSEEncodingError::UnsupportedValue {
                        reason: "category 'gamma' is not in dictionary".to_string(),
                    },
                }
            ))
        ))
    );
    assert_eq!(
        load_typed_query_index_archive_file(&path).unwrap(),
        query_index
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_compacts_tombstoned_archive_and_clears_tombstones() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let query_index_path = temp_archive_path("compact-index", ".fse");
    let tombstone_path = temp_archive_path("compact-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(100)]).unwrap();

    let expected_matches =
        load_typed_query_index_archive_with_tombstones(&query_index_path, &tombstone_path)
            .unwrap()
            .query_row_ids(&plan)
            .unwrap();
    let result = compact_typed_query_index_archive_file(
        &query_index_path,
        &tombstone_path,
        &encoder,
        &builder,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&query_index_path).unwrap();
    let tombstones = load_typed_row_tombstone_archive_file(&tombstone_path).unwrap();
    let after_matches =
        load_typed_query_index_archive_with_tombstones(&query_index_path, &tombstone_path)
            .unwrap()
            .query_row_ids(&plan)
            .unwrap();

    assert_eq!(expected_matches, vec![RowId::new(103)]);
    assert_eq!(result.compaction.base_record_count, 4);
    assert_eq!(result.compaction.tombstone_count, 1);
    assert_eq!(result.compaction.removed_record_count, 1);
    assert_eq!(result.compaction.retained_record_count, 3);
    assert_eq!(
        result.compaction_metadata.payload_kind,
        FSEArchivePayloadKind::TypedQueryIndex
    );
    assert_eq!(result.compaction_metadata.base_record_count, 4);
    assert_eq!(result.compaction_metadata.tombstone_count, 1);
    assert_eq!(result.compaction_metadata.removed_record_count, 1);
    assert_eq!(result.compaction_metadata.retained_record_count, 3);
    assert_eq!(
        result.rebuild_plan.reason,
        FSEArchiveRebuildReason::Compaction
    );
    assert_eq!(
        result.rebuild_plan.operation,
        FSEArchiveRebuildOperationMetadata::Compaction(result.compaction_metadata)
    );
    assert!(result.rebuild_plan.requires_full_archive_rebuild);
    assert_eq!(result.rebuild_plan.resulting_record_count, 3);
    assert_eq!(result.cleared_tombstone_count, 1);
    assert_eq!(result.remaining_tombstone_count, 0);
    assert_eq!(
        loaded.batch().row_ids(),
        &[RowId::new(101), RowId::new(102), RowId::new(103)]
    );
    assert_eq!(tombstones, Vec::new());
    assert_eq!(after_matches, expected_matches);

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_reports_noop_compaction_and_preserves_files() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let query_index_path = temp_archive_path("compact-noop-index", ".fse");
    let tombstone_path = temp_archive_path("compact-noop-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    assert_eq!(
        compact_typed_query_index_archive_file(
            &query_index_path,
            &tombstone_path,
            &encoder,
            &builder,
        ),
        Err(
            FSETypedQueryIndexArchiveCompactionError::CompactionMetadata(
                FSEArchiveCompactionOperationMetadataError::ZeroTombstoneCount
            )
        )
    );
    assert_eq!(
        load_typed_query_index_archive_file(&query_index_path).unwrap(),
        query_index
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_compaction_reports_empty_retained_record_set_and_preserves_files()
{
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index = typed_query_index();
    let tombstone_row_ids = vec![
        RowId::new(100),
        RowId::new(101),
        RowId::new(102),
        RowId::new(103),
    ];
    let query_index_path = temp_archive_path("compact-empty-index", ".fse");
    let tombstone_path = temp_archive_path("compact-empty-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &tombstone_row_ids).unwrap();

    assert_eq!(
        compact_typed_query_index_archive_file(
            &query_index_path,
            &tombstone_path,
            &encoder,
            &builder,
        ),
        Err(FSETypedQueryIndexArchiveCompactionError::Compaction(
            FSETypedQueryIndexCompactionError::EmptyRetainedRecordSet {
                base_record_count: 4,
                tombstone_count: 4
            }
        ))
    );
    assert_eq!(
        load_typed_query_index_archive_file(&query_index_path).unwrap(),
        query_index
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        tombstone_row_ids
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_preserves_clean_archive() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let query_index = typed_query_index();
    let query_index_path = temp_archive_path("maintenance-clean-index", ".fse");
    let tombstone_path = temp_archive_path("maintenance-clean-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let result = maintain_typed_query_index_archive_file(
        &query_index_path,
        &tombstone_path,
        None,
        &encoder,
        &builder,
        &policy,
    )
    .unwrap();

    assert_eq!(
        result.decision.action,
        FSEArchiveMaintenanceAction::NoMaintenance
    );
    assert_eq!(
        result.decision.reason,
        FSEArchiveMaintenanceReason::NoPendingMaintenance
    );
    assert!(!result.decision.requires_archive_write());
    assert_eq!(result.query_index, query_index);
    assert_eq!(result.append_result, None);
    assert_eq!(result.compaction_result, None);
    assert_eq!(
        load_typed_query_index_archive_file(&query_index_path).unwrap(),
        query_index
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_applies_pending_append() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let query_index = typed_query_index();
    let appended = appended_entity_batch(&schema);
    let plan = score_and_class_plan(&schema, &mapping);
    let query_index_path = temp_archive_path("maintenance-append-index", ".fse");
    let tombstone_path = temp_archive_path("maintenance-append-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let result = maintain_typed_query_index_archive_file(
        &query_index_path,
        &tombstone_path,
        Some(&appended),
        &encoder,
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&query_index_path).unwrap();
    let mut matches = loaded.query_row_ids(&plan).unwrap();
    matches.sort();

    assert_eq!(result.decision.action, FSEArchiveMaintenanceAction::Append);
    assert_eq!(
        result.decision.reason,
        FSEArchiveMaintenanceReason::PendingAppendRecords
    );
    assert_eq!(result.decision.input.base_record_count, 4);
    assert_eq!(result.decision.input.pending_append_record_count, 2);
    assert_eq!(result.decision.input.tombstone_count, 0);
    assert_eq!(
        result
            .append_result
            .as_ref()
            .unwrap()
            .append_metadata
            .base_record_count,
        4
    );
    assert_eq!(
        result
            .append_result
            .as_ref()
            .unwrap()
            .append_metadata
            .appended_record_count,
        2
    );
    assert_eq!(result.compaction_result, None);
    assert_eq!(result.query_index, loaded);
    assert_eq!(loaded.batch().len(), 6);
    assert_eq!(
        matches,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_compacts_tombstones() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 9_000).unwrap();
    let query_index = typed_query_index();
    let query_index_path = temp_archive_path("maintenance-compact-index", ".fse");
    let tombstone_path = temp_archive_path("maintenance-compact-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(100)]).unwrap();

    let result = maintain_typed_query_index_archive_file(
        &query_index_path,
        &tombstone_path,
        None,
        &encoder,
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&query_index_path).unwrap();

    assert_eq!(result.decision.action, FSEArchiveMaintenanceAction::Compact);
    assert_eq!(
        result.decision.reason,
        FSEArchiveMaintenanceReason::CompactionTombstoneCountThresholdReached
    );
    assert_eq!(result.append_result, None);
    assert_eq!(
        result
            .compaction_result
            .as_ref()
            .unwrap()
            .compaction
            .removed_record_count,
        1
    );
    assert_eq!(result.query_index, loaded);
    assert_eq!(
        loaded.batch().row_ids(),
        &[RowId::new(101), RowId::new(102), RowId::new(103)]
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_rebuilds_append_and_compaction_work() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 9_000).unwrap();
    let query_index = typed_query_index();
    let appended = appended_entity_batch(&schema);
    let plan = score_and_class_plan(&schema, &mapping);
    let query_index_path = temp_archive_path("maintenance-rebuild-index", ".fse");
    let tombstone_path = temp_archive_path("maintenance-rebuild-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(100)]).unwrap();

    let result = maintain_typed_query_index_archive_file(
        &query_index_path,
        &tombstone_path,
        Some(&appended),
        &encoder,
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&query_index_path).unwrap();
    let mut matches = loaded.query_row_ids(&plan).unwrap();
    matches.sort();

    assert_eq!(result.decision.action, FSEArchiveMaintenanceAction::Rebuild);
    assert_eq!(
        result.decision.reason,
        FSEArchiveMaintenanceReason::AppendAndCompactionThresholdsReached
    );
    assert_eq!(result.decision.input.base_record_count, 4);
    assert_eq!(result.decision.input.pending_append_record_count, 2);
    assert_eq!(result.decision.input.tombstone_count, 1);
    assert_eq!(
        result
            .append_result
            .as_ref()
            .unwrap()
            .append_metadata
            .resulting_record_count,
        6
    );
    assert_eq!(
        result
            .compaction_result
            .as_ref()
            .unwrap()
            .compaction
            .base_record_count,
        6
    );
    assert_eq!(
        result
            .compaction_result
            .as_ref()
            .unwrap()
            .compaction
            .removed_record_count,
        1
    );
    assert_eq!(result.query_index, loaded);
    assert_eq!(
        loaded.batch().row_ids(),
        &[
            RowId::new(101),
            RowId::new(102),
            RowId::new(103),
            RowId::new(104),
            RowId::new(105),
        ]
    );
    assert_eq!(matches, vec![RowId::new(103), RowId::new(104)]);
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_preserves_files_when_combined_rebuild_fails() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 9_000).unwrap();
    let query_index = typed_query_index();
    let appended = appended_entity_batch(&schema);
    let tombstone_row_ids = vec![
        RowId::new(100),
        RowId::new(101),
        RowId::new(102),
        RowId::new(103),
        RowId::new(104),
        RowId::new(105),
    ];
    let query_index_path = temp_archive_path("maintenance-rebuild-failure-index", ".fse");
    let tombstone_path = temp_archive_path("maintenance-rebuild-failure-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &tombstone_row_ids).unwrap();

    assert_eq!(
        maintain_typed_query_index_archive_file(
            &query_index_path,
            &tombstone_path,
            Some(&appended),
            &encoder,
            &builder,
            &policy,
        ),
        Err(FSETypedQueryIndexArchiveMaintenanceError::Compaction(
            FSETypedQueryIndexArchiveCompactionError::Compaction(
                FSETypedQueryIndexCompactionError::EmptyRetainedRecordSet {
                    base_record_count: 6,
                    tombstone_count: 6
                }
            )
        ))
    );
    assert_eq!(
        load_typed_query_index_archive_file(&query_index_path).unwrap(),
        query_index
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        tombstone_row_ids
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_reports_invalid_policy() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy {
        append_rebuild_record_count_threshold: 0,
        compaction_tombstone_count_threshold: 10,
        compaction_tombstone_ratio_threshold_basis_points: 5_000,
    };
    let query_index = typed_query_index();
    let query_index_path = temp_archive_path("maintenance-invalid-policy-index", ".fse");
    let tombstone_path = temp_archive_path("maintenance-invalid-policy-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    assert_eq!(
        maintain_typed_query_index_archive_file(
            &query_index_path,
            &tombstone_path,
            None,
            &encoder,
            &builder,
            &policy,
        ),
        Err(FSETypedQueryIndexArchiveMaintenanceError::Policy(
            FSEArchiveMaintenanceError::ZeroAppendRebuildRecordCountThreshold
        ))
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_reports_invalid_extension_on_save() {
    let query_index = typed_query_index();
    let path = temp_archive_path("wrong-save-extension", ".bin");

    assert_eq!(
        save_typed_query_index_archive_file(&path, &query_index),
        Err(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::InvalidFileExtension { path }
        ))
    );
}

#[test]
fn typed_query_index_archive_file_reports_missing_file_on_load() {
    let path = temp_archive_path("missing-load", ".fse");

    assert_eq!(
        load_typed_query_index_archive_file(&path),
        Err(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Read,
                path,
                kind: io::ErrorKind::NotFound
            }
        ))
    );
}

#[test]
fn typed_query_index_archive_file_reports_payload_header_errors_for_invalid_payload() {
    let path = temp_archive_path("invalid-payload", ".fse");

    fs::write(&path, [0_u8; 4]).unwrap();
    let error = read_typed_query_index_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSETypedQueryIndexArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedEndOfArchive { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_rejects_typed_record_batch_payload_kind() {
    let path = temp_archive_path("wrong-payload-kind", ".fse");
    let bytes = encode_archive_payload(FSEArchivePayloadKind::TypedRecordBatch, &[]);

    fs::write(&path, bytes).unwrap();

    assert_eq!(
        read_typed_query_index_archive_snapshot_file(&path),
        Err(FSETypedQueryIndexArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::UnexpectedPayloadKind {
                expected: FSEArchivePayloadKind::TypedQueryIndex,
                actual: FSEArchivePayloadKind::TypedRecordBatch
            }
        ))
    );

    let _ = fs::remove_file(path);
}

#[test]
fn typed_query_index_archive_file_reports_payload_checksum_mismatch() {
    let path = temp_archive_path("checksum-mismatch", ".fse");
    let bytes = corrupted_archive_payload(FSEArchivePayloadKind::TypedQueryIndex);

    fs::write(&path, bytes).unwrap();
    let error = read_typed_query_index_archive_snapshot_file(&path).unwrap_err();

    assert!(matches!(
        error,
        FSETypedQueryIndexArchiveFileError::Payload(
            FSEArchivePayloadHeaderError::PayloadChecksumMismatch { .. }
        )
    ));

    let _ = fs::remove_file(path);
}

fn typed_query_index() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid typed index should build")
}

fn typed_query_index_with_reverse_category_encoder() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = reverse_category_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid typed index should build")
}

fn score_and_class_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
) -> crate::query::TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
        .with_categorical_encoder(2, class_encoder())
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(10.0),
            FSEValue::Float(20.0),
        ))
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category("alpha".to_string()),
        ))
        .build()
        .expect("valid predicates should produce a plan")
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
        FSEField::new("class", FSEFieldType::Category, false),
        FSEField::new("observed_at", FSEFieldType::TimestampMillis, false),
    ])
}

fn entity_mapping(schema: &FSESchema) -> FSESchemaDimensionMapping {
    FSESchemaDimensionMapping::new(
        schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
            FSEDimensionMapping::new(2, 2),
            FSEDimensionMapping::new(3, 3),
        ],
    )
}

fn entity_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
    ComposedRecordEncoder::new(
        schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(FloatEncoder),
            Box::new(class_encoder()),
            Box::new(TimestampMillisEncoder),
        ],
    )
}

fn reverse_category_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
    ComposedRecordEncoder::new(
        schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(FloatEncoder),
            Box::new(CategoricalDictionaryEncoder::new(vec![
                "beta".to_string(),
                "alpha".to_string(),
            ])),
            Box::new(TimestampMillisEncoder),
        ],
    )
}

fn reverse_category_encoder_metadata() -> FSERecordEncoderMetadata {
    FSERecordEncoderMetadata::new(vec![
        FSEFieldEncoderMetadata::Integer,
        FSEFieldEncoderMetadata::Float,
        FSEFieldEncoderMetadata::CategoryDictionary {
            categories: vec!["beta".to_string(), "alpha".to_string()],
        },
        FSEFieldEncoderMetadata::TimestampMillis,
    ])
}

fn entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(100),
            RowId::new(101),
            RowId::new(102),
            RowId::new(103),
        ],
        vec![
            entity_record(schema, 1, 12.5, "alpha", 1_000),
            entity_record(schema, 2, 12.5, "beta", 1_100),
            entity_record(schema, 3, 25.0, "alpha", 1_200),
            entity_record(schema, 4, 18.0, "alpha", 1_300),
        ],
    )
}

fn appended_entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(104), RowId::new(105)],
        vec![
            entity_record(schema, 5, 16.0, "alpha", 1_400),
            entity_record(schema, 6, 80.0, "beta", 1_500),
        ],
    )
}

fn mismatched_appended_batch() -> FSERecordBatch {
    let schema = FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
    ]);
    let record = FSERecord::new(vec![FSEValue::Integer(5), FSEValue::Float(16.0)], &schema);

    FSERecordBatch::new(schema, vec![RowId::new(104)], vec![record])
}

fn entity_record(
    schema: &FSESchema,
    entity_id: i64,
    score: f64,
    class: &str,
    observed_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(entity_id),
            FSEValue::Float(score),
            FSEValue::Category(class.to_string()),
            FSEValue::TimestampMillis(observed_at),
        ],
        schema,
    )
}

fn class_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["alpha".to_string(), "beta".to_string()])
}

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-typed-query-index-archive-file-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}

fn temp_csv_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-typed-query-index-archive-file-{}-{}.csv",
        std::process::id(),
        name
    ));
    let _ = fs::remove_file(&path);
    path
}
