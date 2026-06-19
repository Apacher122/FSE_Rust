use std::fs;
use std::path::PathBuf;

use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::persistence::{
    FSEArchiveMaintenanceAction, FSEArchiveMaintenancePolicy, FSEArchiveMaintenanceReason,
    FSETypedQueryIndexAppendDeltaArchiveMaintenanceError, FSETypedQueryIndexArchiveCompactionError,
    FSETypedQueryIndexArchiveMaintenanceError, FSETypedQueryIndexCompactionError,
    inspect_typed_query_index_archive_file_maintenance_with_append_batch_archive,
    load_typed_query_index_archive_file, load_typed_record_batch_archive_file,
    load_typed_row_tombstone_archive_file,
    maintain_typed_query_index_archive_file_with_append_batch_archive,
    save_typed_query_index_archive_file, save_typed_record_batch_archive_file,
    save_typed_row_tombstone_archive_file,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedAppendDeltaQueryView, TypedQueryIndex,
    TypedQueryPlanBuilder, TypedRowTombstoneSet,
};

#[test]
fn typed_query_index_archive_file_maintenance_inspects_append_batch_archive_without_writes() {
    let schema = entity_schema();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let query_index = typed_query_index();
    let appended = appended_entity_batch(&schema);
    let query_index_path = temp_archive_path("append-delta-maintenance-inspect-index", ".fse");
    let append_path = temp_archive_path("append-delta-maintenance-inspect-append", ".fse");
    let tombstone_path = temp_archive_path("append-delta-maintenance-inspect-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_record_batch_archive_file(&append_path, &appended).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let decision = inspect_typed_query_index_archive_file_maintenance_with_append_batch_archive(
        &query_index_path,
        &append_path,
        &tombstone_path,
        &policy,
    )
    .unwrap();

    assert_eq!(decision.action, FSEArchiveMaintenanceAction::Append);
    assert_eq!(
        decision.reason,
        FSEArchiveMaintenanceReason::PendingAppendRecords
    );
    assert_eq!(decision.input.base_record_count, 4);
    assert_eq!(decision.input.pending_append_record_count, 2);
    assert_eq!(decision.input.tombstone_count, 0);
    assert_eq!(decision.tombstone_ratio_basis_points, 0);
    assert!(decision.requires_archive_write());
    assert_eq!(
        load_typed_query_index_archive_file(&query_index_path).unwrap(),
        query_index
    );
    assert_eq!(
        load_typed_record_batch_archive_file(&append_path).unwrap(),
        appended
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(append_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_inspects_empty_append_batch_archive() {
    let schema = entity_schema();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let query_index = typed_query_index();
    let appended = FSERecordBatch::new(schema.clone(), Vec::new(), Vec::new());
    let query_index_path =
        temp_archive_path("append-delta-maintenance-inspect-empty-index", ".fse");
    let append_path = temp_archive_path("append-delta-maintenance-inspect-empty-append", ".fse");
    let tombstone_path =
        temp_archive_path("append-delta-maintenance-inspect-empty-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_record_batch_archive_file(&append_path, &appended).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let decision = inspect_typed_query_index_archive_file_maintenance_with_append_batch_archive(
        &query_index_path,
        &append_path,
        &tombstone_path,
        &policy,
    )
    .unwrap();

    assert_eq!(decision.action, FSEArchiveMaintenanceAction::NoMaintenance);
    assert_eq!(
        decision.reason,
        FSEArchiveMaintenanceReason::NoPendingMaintenance
    );
    assert_eq!(decision.input.base_record_count, 4);
    assert_eq!(decision.input.pending_append_record_count, 0);
    assert_eq!(decision.input.tombstone_count, 0);
    assert!(!decision.requires_archive_write());
    assert_eq!(
        load_typed_query_index_archive_file(&query_index_path).unwrap(),
        query_index
    );
    assert_eq!(
        load_typed_record_batch_archive_file(&append_path).unwrap(),
        appended
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(append_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_consumes_append_batch_archive() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let query_index = typed_query_index();
    let appended = appended_entity_batch(&schema);
    let plan = score_and_class_plan(&schema, &mapping);
    let query_index_path = temp_archive_path("append-delta-maintenance-index", ".fse");
    let append_path = temp_archive_path("append-delta-maintenance-append", ".fse");
    let tombstone_path = temp_archive_path("append-delta-maintenance-tombstones", ".fse");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended).unwrap();
    let mut expected_matches = view.query_row_ids(&plan).unwrap();
    expected_matches.sort();

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_record_batch_archive_file(&append_path, &appended).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let result = maintain_typed_query_index_archive_file_with_append_batch_archive(
        &query_index_path,
        &append_path,
        &tombstone_path,
        &encoder,
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&query_index_path).unwrap();
    let cleared_append = load_typed_record_batch_archive_file(&append_path).unwrap();
    let mut actual_matches = loaded.query_row_ids(&plan).unwrap();
    actual_matches.sort();

    assert_eq!(result.decision.action, FSEArchiveMaintenanceAction::Append);
    assert_eq!(
        result.decision.reason,
        FSEArchiveMaintenanceReason::PendingAppendRecords
    );
    assert!(result.append_result.is_some());
    assert_eq!(result.compaction_result, None);
    assert_eq!(result.query_index, loaded);
    assert_eq!(actual_matches, expected_matches);
    assert!(cleared_append.is_empty());
    assert_eq!(cleared_append.schema(), &schema);
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(append_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_compacts_append_and_tombstone_archives() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 9_000).unwrap();
    let query_index = typed_query_index();
    let appended = appended_entity_batch(&schema);
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(100)]);
    let plan = score_and_class_plan(&schema, &mapping);
    let query_index_path = temp_archive_path("append-delta-maintenance-rebuild-index", ".fse");
    let append_path = temp_archive_path("append-delta-maintenance-rebuild-append", ".fse");
    let tombstone_path = temp_archive_path("append-delta-maintenance-rebuild-tombstones", ".fse");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended).unwrap();
    let mut expected_matches = view
        .query_row_ids_excluding_tombstones(&plan, &tombstones)
        .unwrap();
    expected_matches.sort();

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_record_batch_archive_file(&append_path, &appended).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, tombstones.row_ids()).unwrap();

    let result = maintain_typed_query_index_archive_file_with_append_batch_archive(
        &query_index_path,
        &append_path,
        &tombstone_path,
        &encoder,
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&query_index_path).unwrap();
    let cleared_append = load_typed_record_batch_archive_file(&append_path).unwrap();
    let mut actual_matches = loaded.query_row_ids(&plan).unwrap();
    actual_matches.sort();

    assert_eq!(result.decision.action, FSEArchiveMaintenanceAction::Rebuild);
    assert_eq!(
        result.decision.reason,
        FSEArchiveMaintenanceReason::AppendAndCompactionThresholdsReached
    );
    assert!(result.append_result.is_some());
    assert!(result.compaction_result.is_some());
    assert_eq!(result.query_index, loaded);
    assert_eq!(actual_matches, expected_matches);
    assert_eq!(loaded.batch().len(), 5);
    assert!(cleared_append.is_empty());
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(append_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_file_maintenance_preserves_append_archive_on_failure() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
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
    let query_index_path = temp_archive_path("append-delta-maintenance-failure-index", ".fse");
    let append_path = temp_archive_path("append-delta-maintenance-failure-append", ".fse");
    let tombstone_path = temp_archive_path("append-delta-maintenance-failure-tombstones", ".fse");

    save_typed_query_index_archive_file(&query_index_path, &query_index).unwrap();
    save_typed_record_batch_archive_file(&append_path, &appended).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &tombstone_row_ids).unwrap();

    let error = maintain_typed_query_index_archive_file_with_append_batch_archive(
        &query_index_path,
        &append_path,
        &tombstone_path,
        &encoder,
        &builder,
        &policy,
    )
    .unwrap_err();

    assert_eq!(
        error,
        FSETypedQueryIndexAppendDeltaArchiveMaintenanceError::Maintenance(
            FSETypedQueryIndexArchiveMaintenanceError::Compaction(
                FSETypedQueryIndexArchiveCompactionError::Compaction(
                    FSETypedQueryIndexCompactionError::EmptyRetainedRecordSet {
                        base_record_count: 6,
                        tombstone_count: 6,
                    },
                ),
            ),
        )
    );
    assert_eq!(
        load_typed_query_index_archive_file(&query_index_path).unwrap(),
        query_index
    );
    assert_eq!(
        load_typed_record_batch_archive_file(&append_path).unwrap(),
        appended
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        tombstone_row_ids
    );

    let _ = fs::remove_file(query_index_path);
    let _ = fs::remove_file(append_path);
    let _ = fs::remove_file(tombstone_path);
}

fn typed_query_index() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);

    TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid typed index should build")
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

fn builder() -> FSEBuilder {
    FSEBuilder::new(BuildConfig::new(2, 8))
}

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-typed-query-maintenance-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}
