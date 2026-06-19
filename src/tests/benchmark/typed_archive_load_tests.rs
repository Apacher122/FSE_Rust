use std::fs;

use crate::benchmark::reports::{
    RepeatedTimingConfig, compare_typed_archive_append_delta_maintenance_execution_repeated,
    compare_typed_archive_append_rebuild_execution_repeated,
    compare_typed_archive_compaction_execution_repeated,
    compare_typed_archive_load_execution_repeated,
    compare_typed_archive_maintenance_execution_repeated,
};
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
    FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveMaintenanceAction, FSEArchiveMaintenancePolicy,
    FSEArchiveMaintenanceReason,
};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryPlan};

#[test]
fn typed_archive_load_timing_reports_exact_loaded_execution() {
    let fixture = typed_fixture();
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(11.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");
    let timing_config = RepeatedTimingConfig::new(3);
    let path = archive_path("typed_archive_load_timing_reports_exact_loaded_execution");

    let _ = fs::remove_file(&path);

    let report = compare_typed_archive_load_execution_repeated(
        &path,
        &fixture.query_index,
        &plan,
        &timing_config,
    )
    .expect("typed archive timing should execute");

    assert_eq!(report.matched_records, 2);
    assert_eq!(report.in_memory_timing.iterations, 3);
    assert_eq!(report.warm_loaded_timing.iterations, 3);
    assert_eq!(report.cold_loaded_timing.iterations, 3);
    assert!(report.warm_loaded_to_in_memory_ratio >= 0.0);
    assert!(report.cold_loaded_to_in_memory_ratio >= 0.0);
    assert!(path.exists());

    fs::remove_file(path).expect("test archive file should be removable");
}

#[test]
fn typed_archive_append_rebuild_timing_reports_rebuilt_archive() {
    let fixture = typed_fixture();
    let appended = appended_entity_batch(&fixture.schema);
    let encoder = entity_encoder(&fixture.schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(12.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");
    let timing_config = RepeatedTimingConfig::new(3);
    let path = archive_path("typed_archive_append_rebuild_timing_reports_rebuilt_archive");

    let _ = fs::remove_file(&path);

    let report = compare_typed_archive_append_rebuild_execution_repeated(
        &path,
        &fixture.query_index,
        &appended,
        &encoder,
        &builder,
        &plan,
        &timing_config,
    )
    .expect("typed archive append rebuild timing should execute");

    assert_eq!(report.base_record_count, 4);
    assert_eq!(report.appended_record_count, 2);
    assert_eq!(report.resulting_record_count, 6);
    assert!(report.archive_bytes_before_append > 0);
    assert!(report.archive_bytes_after_append >= report.archive_bytes_before_append);
    assert_eq!(
        report.archive_byte_growth,
        report
            .archive_bytes_after_append
            .saturating_sub(report.archive_bytes_before_append)
    );
    assert_eq!(report.matched_records_after_append, 4);
    assert_eq!(report.append_rebuild_timing.iterations, 3);
    assert!(path.exists());

    fs::remove_file(path).expect("test archive file should be removable");
}

#[test]
fn typed_archive_compaction_timing_reports_compacted_archive() {
    let fixture = typed_fixture();
    let tombstone_row_ids = vec![RowId::new(10)];
    let encoder = entity_encoder(&fixture.schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(11.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");
    let timing_config = RepeatedTimingConfig::new(3);
    let query_archive_path =
        archive_path("typed_archive_compaction_timing_reports_compacted_archive-index");
    let tombstone_archive_path =
        archive_path("typed_archive_compaction_timing_reports_compacted_archive-tombstones");

    let _ = fs::remove_file(&query_archive_path);
    let _ = fs::remove_file(&tombstone_archive_path);

    let report = compare_typed_archive_compaction_execution_repeated(
        &query_archive_path,
        &tombstone_archive_path,
        &fixture.query_index,
        &tombstone_row_ids,
        &encoder,
        &builder,
        &plan,
        &timing_config,
    )
    .expect("typed archive compaction timing should execute");

    assert_eq!(report.base_record_count, 4);
    assert_eq!(report.tombstone_count, 1);
    assert_eq!(report.removed_record_count, 1);
    assert_eq!(report.retained_record_count, 3);
    assert!(report.query_archive_bytes_before_compaction > 0);
    assert!(report.query_archive_bytes_after_compaction > 0);
    assert!(report.tombstone_archive_bytes_before_compaction > 0);
    assert!(report.tombstone_archive_bytes_after_compaction > 0);
    assert_eq!(
        report.query_archive_byte_delta,
        i128::from(report.query_archive_bytes_after_compaction)
            - i128::from(report.query_archive_bytes_before_compaction)
    );
    assert_eq!(
        report.tombstone_archive_byte_delta,
        i128::from(report.tombstone_archive_bytes_after_compaction)
            - i128::from(report.tombstone_archive_bytes_before_compaction)
    );
    assert_eq!(
        report.logical_archive_bytes_before_compaction,
        report.query_archive_bytes_before_compaction
            + report.tombstone_archive_bytes_before_compaction
    );
    assert_eq!(
        report.logical_archive_bytes_after_compaction,
        report.query_archive_bytes_after_compaction
            + report.tombstone_archive_bytes_after_compaction
    );
    assert_eq!(
        report.logical_archive_byte_delta,
        i128::from(report.logical_archive_bytes_after_compaction)
            - i128::from(report.logical_archive_bytes_before_compaction)
    );
    assert_eq!(report.matched_records_after_compaction, 1);
    assert_eq!(report.compaction_timing.iterations, 3);
    assert!(query_archive_path.exists());
    assert!(tombstone_archive_path.exists());

    fs::remove_file(query_archive_path).expect("test query archive file should be removable");
    fs::remove_file(tombstone_archive_path)
        .expect("test tombstone archive file should be removable");
}

#[test]
fn typed_archive_maintenance_timing_reports_policy_driven_rebuild() {
    let fixture = typed_fixture();
    let appended = appended_entity_batch(&fixture.schema);
    let tombstone_row_ids = vec![RowId::new(10)];
    let encoder = entity_encoder(&fixture.schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 9_000)
        .expect("maintenance policy should be valid");
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(12.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");
    let timing_config = RepeatedTimingConfig::new(3);
    let query_archive_path =
        archive_path("typed_archive_maintenance_timing_reports_policy_driven_rebuild-index");
    let tombstone_archive_path =
        archive_path("typed_archive_maintenance_timing_reports_policy_driven_rebuild-tombstones");

    let _ = fs::remove_file(&query_archive_path);
    let _ = fs::remove_file(&tombstone_archive_path);

    let report = compare_typed_archive_maintenance_execution_repeated(
        &query_archive_path,
        &tombstone_archive_path,
        &fixture.query_index,
        Some(&appended),
        &tombstone_row_ids,
        &encoder,
        &builder,
        &policy,
        &plan,
        &timing_config,
    )
    .expect("typed archive maintenance timing should execute");

    assert_eq!(report.base_record_count, 4);
    assert_eq!(report.pending_append_record_count, 2);
    assert_eq!(report.tombstone_count, 1);
    assert_eq!(report.selected_action, FSEArchiveMaintenanceAction::Rebuild);
    assert_eq!(
        report.selected_reason,
        FSEArchiveMaintenanceReason::AppendAndCompactionThresholdsReached
    );
    assert_eq!(report.tombstone_ratio_basis_points, 2_500);
    assert_eq!(report.resulting_record_count, 5);
    assert!(report.query_archive_bytes_before_maintenance > 0);
    assert!(report.query_archive_bytes_after_maintenance > 0);
    assert!(report.tombstone_archive_bytes_before_maintenance > 0);
    assert!(report.tombstone_archive_bytes_after_maintenance > 0);
    assert_eq!(
        report.query_archive_byte_delta,
        i128::from(report.query_archive_bytes_after_maintenance)
            - i128::from(report.query_archive_bytes_before_maintenance)
    );
    assert_eq!(
        report.tombstone_archive_byte_delta,
        i128::from(report.tombstone_archive_bytes_after_maintenance)
            - i128::from(report.tombstone_archive_bytes_before_maintenance)
    );
    assert_eq!(
        report.logical_archive_bytes_before_maintenance,
        report.query_archive_bytes_before_maintenance
            + report.tombstone_archive_bytes_before_maintenance
    );
    assert_eq!(
        report.logical_archive_bytes_after_maintenance,
        report.query_archive_bytes_after_maintenance
            + report.tombstone_archive_bytes_after_maintenance
    );
    assert_eq!(
        report.logical_archive_byte_delta,
        i128::from(report.logical_archive_bytes_after_maintenance)
            - i128::from(report.logical_archive_bytes_before_maintenance)
    );
    assert_eq!(report.matched_records_after_maintenance, 3);
    assert_eq!(report.maintenance_timing.iterations, 3);
    assert!(query_archive_path.exists());
    assert!(tombstone_archive_path.exists());

    fs::remove_file(query_archive_path).expect("test query archive file should be removable");
    fs::remove_file(tombstone_archive_path)
        .expect("test tombstone archive file should be removable");
}

#[test]
fn typed_archive_append_delta_maintenance_timing_reports_persisted_delta_rebuild() {
    let fixture = typed_fixture();
    let appended = appended_entity_batch(&fixture.schema);
    let tombstone_row_ids = vec![RowId::new(10)];
    let encoder = entity_encoder(&fixture.schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 9_000)
        .expect("maintenance policy should be valid");
    let predicate = FSEPredicate::range(
        FSEPredicateField::name("score"),
        FSEValue::Float(10.0),
        FSEValue::Float(12.0),
    );
    let plan = TypedQueryPlan::numeric(&predicate, &fixture.schema, &fixture.mapping)
        .expect("numeric predicate should produce a plan");
    let timing_config = RepeatedTimingConfig::new(3);
    let query_archive_path = archive_path(
        "typed_archive_append_delta_maintenance_timing_reports_persisted_delta_rebuild-index",
    );
    let append_archive_path = archive_path(
        "typed_archive_append_delta_maintenance_timing_reports_persisted_delta_rebuild-append",
    );
    let tombstone_archive_path = archive_path(
        "typed_archive_append_delta_maintenance_timing_reports_persisted_delta_rebuild-tombstones",
    );

    let _ = fs::remove_file(&query_archive_path);
    let _ = fs::remove_file(&append_archive_path);
    let _ = fs::remove_file(&tombstone_archive_path);

    let report = compare_typed_archive_append_delta_maintenance_execution_repeated(
        &query_archive_path,
        &append_archive_path,
        &tombstone_archive_path,
        &fixture.query_index,
        &appended,
        &tombstone_row_ids,
        &encoder,
        &builder,
        &policy,
        &plan,
        &timing_config,
    )
    .expect("append-delta archive maintenance timing should execute");

    assert_eq!(report.base_record_count, 4);
    assert_eq!(report.pending_append_record_count, 2);
    assert_eq!(report.tombstone_count, 1);
    assert_eq!(report.selected_action, FSEArchiveMaintenanceAction::Rebuild);
    assert_eq!(
        report.selected_reason,
        FSEArchiveMaintenanceReason::AppendAndCompactionThresholdsReached
    );
    assert_eq!(report.tombstone_ratio_basis_points, 2_500);
    assert!(report.maintenance_status_requires_archive_write);
    assert_eq!(report.resulting_record_count, 5);
    assert!(report.query_archive_bytes_before_maintenance > 0);
    assert!(report.query_archive_bytes_after_maintenance > 0);
    assert!(report.append_archive_bytes_before_maintenance > 0);
    assert!(report.append_archive_bytes_after_maintenance > 0);
    assert!(report.tombstone_archive_bytes_before_maintenance > 0);
    assert!(report.tombstone_archive_bytes_after_maintenance > 0);
    assert_eq!(
        report.query_archive_byte_delta,
        i128::from(report.query_archive_bytes_after_maintenance)
            - i128::from(report.query_archive_bytes_before_maintenance)
    );
    assert_eq!(
        report.append_archive_byte_delta,
        i128::from(report.append_archive_bytes_after_maintenance)
            - i128::from(report.append_archive_bytes_before_maintenance)
    );
    assert_eq!(
        report.tombstone_archive_byte_delta,
        i128::from(report.tombstone_archive_bytes_after_maintenance)
            - i128::from(report.tombstone_archive_bytes_before_maintenance)
    );
    assert_eq!(
        report.logical_archive_bytes_before_maintenance,
        report.query_archive_bytes_before_maintenance
            + report.append_archive_bytes_before_maintenance
            + report.tombstone_archive_bytes_before_maintenance
    );
    assert_eq!(
        report.logical_archive_bytes_after_maintenance,
        report.query_archive_bytes_after_maintenance
            + report.append_archive_bytes_after_maintenance
            + report.tombstone_archive_bytes_after_maintenance
    );
    assert_eq!(
        report.logical_archive_byte_delta,
        i128::from(report.logical_archive_bytes_after_maintenance)
            - i128::from(report.logical_archive_bytes_before_maintenance)
    );
    assert_eq!(report.matched_records_after_maintenance, 3);
    assert_eq!(report.maintenance_timing.iterations, 3);
    assert!(query_archive_path.exists());
    assert!(append_archive_path.exists());
    assert!(tombstone_archive_path.exists());

    fs::remove_file(query_archive_path).expect("test query archive file should be removable");
    fs::remove_file(append_archive_path).expect("test append archive file should be removable");
    fs::remove_file(tombstone_archive_path)
        .expect("test tombstone archive file should be removable");
}

struct TypedFixture {
    schema: FSESchema,
    mapping: FSESchemaDimensionMapping,
    query_index: TypedQueryIndex,
}

fn typed_fixture() -> TypedFixture {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder).expect("valid input should build");

    TypedFixture {
        schema,
        mapping,
        query_index,
    }
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
        FSEField::new("status", FSEFieldType::Category, false),
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
            Box::new(status_encoder()),
            Box::new(TimestampMillisEncoder),
        ],
    )
}

fn entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(10),
            RowId::new(11),
            RowId::new(12),
            RowId::new(13),
        ],
        vec![
            entity_record(schema, 42, 10.0, "open", 1_735_689_600_000),
            entity_record(schema, 43, 11.0, "open", 1_735_689_650_000),
            entity_record(schema, 44, 1000.0, "closed", 1_735_689_700_000),
            entity_record(schema, 45, 1001.0, "closed", 1_735_689_750_000),
        ],
    )
}

fn appended_entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(14), RowId::new(15)],
        vec![
            entity_record(schema, 46, 10.5, "open", 1_735_689_800_000),
            entity_record(schema, 47, 12.0, "closed", 1_735_689_850_000),
        ],
    )
}

fn entity_record(
    schema: &FSESchema,
    entity_id: i64,
    score: f64,
    status: &str,
    observed_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(entity_id),
            FSEValue::Float(score),
            FSEValue::Category(status.to_string()),
            FSEValue::TimestampMillis(observed_at),
        ],
        schema,
    )
}

fn status_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["open".to_string(), "closed".to_string()])
}

fn archive_path(test_name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "fse-{test_name}-{}{}",
        std::process::id(),
        FSE_ARCHIVE_FILE_EXTENSION
    ))
}
