use std::fs;
use std::io;
use std::path::PathBuf;

use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSECsvFileImportError, FSECsvImportError, FSECsvImportOptions, FSECsvSchemaInferenceOptions,
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecordBatchError, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, ComposedRecordEncoderFromBatchError,
    FSEFieldEncoderMetadata, FloatEncoder, IntegerEncoder, TimestampMillisEncoder,
};
use crate::import::{
    FSECsvArchiveImportError, FSECsvArchiveMaintenanceImportError, FSECsvArchiveQueryContextError,
    FSECsvArchiveQueryError, FSECsvInferredArchiveImportError, FSECsvTombstoneImportError,
    FSECsvTombstoneMaintenanceImportError, FSECsvTombstonedArchiveQueryContextError,
    append_typed_query_index_archive_from_csv_file,
    append_typed_query_index_archive_from_csv_file_with_archive_metadata,
    append_typed_row_tombstone_archive_from_csv_file,
    build_typed_query_index_archive_from_csv_file,
    build_typed_query_index_archive_from_inferred_csv_file,
    load_csv_tombstoned_typed_query_index_archive_context,
    load_csv_typed_query_index_archive_context, maintain_typed_query_index_archive_from_csv_file,
    maintain_typed_query_index_archive_from_csv_file_with_archive_metadata,
    maintain_typed_query_index_archive_from_csv_tombstone_file,
};
use crate::persistence::{
    FSEArchiveFileOperation, FSEArchiveMaintenanceAction, FSEArchiveMaintenanceError,
    FSEArchiveMaintenancePolicy, FSEArchiveMaintenanceReason, FSEArchivePayloadKind,
    FSEArchiveRebuildOperationMetadata, FSEArchiveRebuildReason, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFileError, FSETypedQueryIndexArchiveMaintenanceError,
    FSETypedRowTombstoneArchiveError, load_typed_query_index_archive_file,
    load_typed_query_index_archive_file_with_encoder_metadata,
    load_typed_query_index_archive_with_tombstones, load_typed_row_tombstone_archive_file,
    save_typed_row_tombstone_archive_file,
};
use crate::query::{
    FSEPredicate, FSEPredicateError, FSEPredicateField, TypedQueryIndexAppendError,
    TypedQueryPlanBuilder, TypedQueryPlanError,
};

#[test]
fn csv_archive_import_builds_queryable_fse_file() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("queryable");
    let archive_path = temp_archive_path("queryable", ".fse");

    fs::write(&csv_path, entity_csv()).unwrap();

    let query_index = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&archive_path).unwrap();
    let plan = score_and_class_plan(&schema, &mapping);

    assert!(archive_path.exists());
    assert_eq!(loaded, query_index);
    assert_eq!(
        loaded.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_import_builds_queryable_fse_file_from_inferred_schema() {
    let expected_schema = entity_schema();
    let mapping = FSESchemaDimensionMapping::identity(&expected_schema);
    let builder = builder();
    let csv_path = temp_csv_path("queryable-inferred");
    let archive_path = temp_archive_path("queryable-inferred", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();

    let result = build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file_with_encoder_metadata(&archive_path).unwrap();
    let plan = TypedQueryPlanBuilder::new(&expected_schema, &mapping)
        .try_with_record_encoder_metadata(&loaded.record_encoder_metadata)
        .unwrap()
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
        .unwrap();

    assert!(archive_path.exists());
    assert_eq!(result.schema, expected_schema);
    assert_eq!(loaded.query_index, result.query_index);
    assert_eq!(
        result.record_encoder_metadata.fields(),
        &[
            FSEFieldEncoderMetadata::Integer,
            FSEFieldEncoderMetadata::Float,
            FSEFieldEncoderMetadata::CategoryDictionary {
                categories: vec!["alpha".to_string(), "beta".to_string()],
            },
            FSEFieldEncoderMetadata::TimestampMillis,
        ]
    );
    assert_eq!(
        loaded.record_encoder_metadata.fields(),
        result.record_encoder_metadata.fields()
    );
    assert_eq!(
        loaded.query_index.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_query_context_loads_plan_builder_from_fse_file() {
    let expected_schema = entity_schema();
    let builder = builder();
    let csv_path = temp_csv_path("query-context");
    let archive_path = temp_archive_path("query-context", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();

    let imported = build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();

    let context = load_csv_typed_query_index_archive_context(&archive_path).unwrap();
    let plan = context
        .try_plan_builder()
        .unwrap()
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
        .unwrap();

    assert_eq!(context.schema, expected_schema);
    assert_eq!(
        context.mapping,
        FSESchemaDimensionMapping::identity(&expected_schema)
    );
    assert_eq!(
        context.record_encoder_metadata.fields(),
        imported.record_encoder_metadata.fields()
    );
    assert_eq!(context.query_index, imported.query_index);
    assert_eq!(
        context.query_index.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_query_context_queries_predicates_from_fse_file() {
    let builder = builder();
    let csv_path = temp_csv_path("query-context-query");
    let archive_path = temp_archive_path("query-context-query", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();

    build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();

    let context = load_csv_typed_query_index_archive_context(&archive_path).unwrap();
    let row_ids = context.query_row_ids(score_and_class_predicates()).unwrap();
    let rows = context.query_rows(score_and_class_predicates()).unwrap();

    assert_eq!(row_ids, vec![RowId::new(100), RowId::new(103)]);
    assert_eq!(
        rows.iter().map(|row| row.row_id()).collect::<Vec<_>>(),
        row_ids
    );
    assert_eq!(
        rows[0].record().value_named(&context.schema, "class"),
        Some(&FSEValue::Category("alpha".to_string()))
    );
    assert_eq!(
        context.count_matches(score_and_class_predicates()).unwrap(),
        2
    );
    assert!(context.has_match(score_and_class_predicates()).unwrap());
    assert!(
        !context
            .has_match(vec![FSEPredicate::range(
                FSEPredicateField::name("score"),
                FSEValue::Float(90.0),
                FSEValue::Float(100.0),
            )])
            .unwrap()
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_tombstoned_archive_query_context_excludes_tombstoned_rows_from_fse_file() {
    let builder = builder();
    let csv_path = temp_csv_path("tombstoned-query-context-query");
    let archive_path = temp_archive_path("tombstoned-query-context-query", ".fse");
    let tombstone_path = temp_archive_path("tombstoned-query-context-query-tombstones", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();

    build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(103)]).unwrap();

    let context = load_csv_typed_query_index_archive_context(&archive_path).unwrap();
    let tombstoned =
        load_csv_tombstoned_typed_query_index_archive_context(&archive_path, &tombstone_path)
            .unwrap();
    let row_ids = tombstoned
        .query_row_ids(score_and_class_predicates())
        .unwrap();
    let rows = tombstoned.query_rows(score_and_class_predicates()).unwrap();

    assert_eq!(
        context.query_row_ids(score_and_class_predicates()).unwrap(),
        vec![RowId::new(100), RowId::new(103)]
    );
    assert_eq!(tombstoned.tombstones.row_ids(), &[RowId::new(103)]);
    assert_eq!(row_ids, vec![RowId::new(100)]);
    assert_eq!(
        rows.iter().map(|row| row.row_id()).collect::<Vec<_>>(),
        row_ids
    );
    assert_eq!(
        tombstoned
            .count_matches(score_and_class_predicates())
            .unwrap(),
        1
    );
    assert!(tombstoned.has_match(score_and_class_predicates()).unwrap());
    assert!(
        !tombstoned
            .has_match(vec![
                FSEPredicate::range(
                    FSEPredicateField::name("score"),
                    FSEValue::Float(18.0),
                    FSEValue::Float(18.0),
                ),
                FSEPredicate::equals(
                    FSEPredicateField::name("class"),
                    FSEValue::Category("alpha".to_string()),
                ),
            ])
            .unwrap()
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_tombstoned_archive_query_context_reports_tombstone_load_error() {
    let builder = builder();
    let csv_path = temp_csv_path("tombstoned-query-context-load-error");
    let archive_path = temp_archive_path("tombstoned-query-context-load-error", ".fse");
    let tombstone_path =
        temp_archive_path("tombstoned-query-context-load-error-tombstones", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();

    build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();

    let error =
        load_csv_tombstoned_typed_query_index_archive_context(&archive_path, &tombstone_path)
            .unwrap_err();

    assert!(matches!(
        error,
        FSECsvTombstonedArchiveQueryContextError::Tombstones(_)
    ));

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_query_context_reports_predicate_plan_error() {
    let builder = builder();
    let csv_path = temp_csv_path("query-context-plan-error");
    let archive_path = temp_archive_path("query-context-plan-error", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();

    build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();

    let context = load_csv_typed_query_index_archive_context(&archive_path).unwrap();
    let error = context
        .query_row_ids(vec![FSEPredicate::equals(
            FSEPredicateField::name("unknown"),
            FSEValue::Integer(1),
        )])
        .unwrap_err();

    match error {
        FSECsvArchiveQueryError::Plan(TypedQueryPlanError::Predicate(
            FSEPredicateError::UnknownFieldName { name },
        )) => assert_eq!(name, "unknown"),
        other => panic!("expected unknown field query plan error, got {other}"),
    }

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_query_context_reports_archive_load_error() {
    let archive_path = temp_archive_path("query-context-invalid-extension", ".bin");

    let error = load_csv_typed_query_index_archive_context(&archive_path).unwrap_err();

    assert!(matches!(
        error,
        FSECsvArchiveQueryContextError::Archive(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::InvalidFileExtension { .. }
        ))
    ));
}

#[test]
fn csv_archive_import_reports_missing_inferred_schema_override() {
    let builder = builder();
    let csv_path = temp_csv_path("missing-inference-override");
    let archive_path = temp_archive_path("missing-inference-override", ".fse");
    let schema_options =
        FSECsvSchemaInferenceOptions::new().with_field_type("missing", FSEFieldType::Category);

    fs::write(&csv_path, entity_csv()).unwrap();

    let error = build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap_err();

    match error {
        FSECsvInferredArchiveImportError::Csv(FSECsvFileImportError::Import(
            FSECsvImportError::MissingSchemaInferenceOverride { field },
        )) => assert_eq!(field, "missing"),
        other => panic!("expected missing schema inference override, got {other}"),
    }
    assert!(!archive_path.exists());

    let _ = fs::remove_file(csv_path);
}

#[test]
fn csv_archive_import_reports_unsupported_inferred_text_encoder() {
    let builder = builder();
    let csv_path = temp_csv_path("unsupported-inferred-text");
    let archive_path = temp_archive_path("unsupported-inferred-text", ".fse");

    fs::write(&csv_path, entity_csv()).unwrap();

    let error = build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &FSECsvSchemaInferenceOptions::new(),
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap_err();

    match error {
        FSECsvInferredArchiveImportError::Encoder(
            ComposedRecordEncoderFromBatchError::UnsupportedFieldType {
                name, field_type, ..
            },
        ) => {
            assert_eq!(name, "class");
            assert_eq!(field_type, FSEFieldType::Text);
        }
        other => panic!("expected unsupported inferred text encoder, got {other}"),
    }
    assert!(!archive_path.exists());

    let _ = fs::remove_file(csv_path);
}

#[test]
fn csv_archive_import_appends_csv_records_to_fse_file() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("append-base");
    let append_csv_path = temp_csv_path("append-records");
    let archive_path = temp_archive_path("append-records", ".fse");

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, appended_entity_csv()).unwrap();

    build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();

    let result = append_typed_query_index_archive_from_csv_file(
        &append_csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&archive_path).unwrap();
    let plan = score_and_class_plan(&schema, &mapping);
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

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_import_appends_csv_records_using_archive_metadata() {
    let expected_schema = entity_schema();
    let mapping = FSESchemaDimensionMapping::identity(&expected_schema);
    let builder = builder();
    let csv_path = temp_csv_path("archive-metadata-append-base");
    let append_csv_path = temp_csv_path("archive-metadata-append-records");
    let archive_path = temp_archive_path("archive-metadata-append-records", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, appended_entity_csv()).unwrap();

    let imported = build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();

    let result = append_typed_query_index_archive_from_csv_file_with_archive_metadata(
        &append_csv_path,
        &archive_path,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file_with_encoder_metadata(&archive_path).unwrap();
    let plan = TypedQueryPlanBuilder::new(&expected_schema, &mapping)
        .try_with_record_encoder_metadata(&loaded.record_encoder_metadata)
        .unwrap()
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
        .unwrap();
    let mut matches = loaded.query_index.query_row_ids(&plan).unwrap();
    matches.sort();

    assert_eq!(result.append_metadata.base_record_count, 4);
    assert_eq!(result.append_metadata.appended_record_count, 2);
    assert_eq!(result.append_metadata.resulting_record_count, 6);
    assert_eq!(result.query_index, loaded.query_index);
    assert_eq!(
        loaded.record_encoder_metadata.fields(),
        imported.record_encoder_metadata.fields()
    );
    assert_eq!(
        matches,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_import_maintenance_appends_csv_records_to_fse_file() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let csv_path = temp_csv_path("maintenance-base");
    let append_csv_path = temp_csv_path("maintenance-append");
    let archive_path = temp_archive_path("maintenance-append", ".fse");
    let tombstone_path = temp_archive_path("maintenance-tombstones", ".fse");

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, appended_entity_csv()).unwrap();

    build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let result = maintain_typed_query_index_archive_from_csv_file(
        &append_csv_path,
        &archive_path,
        &tombstone_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file(&archive_path).unwrap();
    let tombstones = load_typed_row_tombstone_archive_file(&tombstone_path).unwrap();
    let plan = score_and_class_plan(&schema, &mapping);
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
            .resulting_record_count,
        6
    );
    assert_eq!(result.compaction_result, None);
    assert_eq!(result.query_index, loaded);
    assert_eq!(tombstones, Vec::new());
    assert_eq!(loaded.batch().len(), 6);
    assert_eq!(
        matches,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_archive_import_maintenance_uses_archive_metadata() {
    let expected_schema = entity_schema();
    let mapping = FSESchemaDimensionMapping::identity(&expected_schema);
    let builder = builder();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let csv_path = temp_csv_path("maintenance-archive-metadata-base");
    let append_csv_path = temp_csv_path("maintenance-archive-metadata-append");
    let archive_path = temp_archive_path("maintenance-archive-metadata-append", ".fse");
    let tombstone_path = temp_archive_path("maintenance-archive-metadata-tombstones", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, appended_entity_csv()).unwrap();

    let imported = build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let result = maintain_typed_query_index_archive_from_csv_file_with_archive_metadata(
        &append_csv_path,
        &archive_path,
        &tombstone_path,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file_with_encoder_metadata(&archive_path).unwrap();
    let tombstones = load_typed_row_tombstone_archive_file(&tombstone_path).unwrap();
    let plan = TypedQueryPlanBuilder::new(&expected_schema, &mapping)
        .try_with_record_encoder_metadata(&loaded.record_encoder_metadata)
        .unwrap()
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
        .unwrap();
    let mut matches = loaded.query_index.query_row_ids(&plan).unwrap();
    matches.sort();

    assert_eq!(result.decision.action, FSEArchiveMaintenanceAction::Append);
    assert_eq!(result.decision.input.base_record_count, 4);
    assert_eq!(result.decision.input.pending_append_record_count, 2);
    assert_eq!(result.decision.input.tombstone_count, 0);
    assert_eq!(result.compaction_result, None);
    assert_eq!(result.query_index, loaded.query_index);
    assert_eq!(tombstones, Vec::new());
    assert_eq!(
        loaded.record_encoder_metadata.fields(),
        imported.record_encoder_metadata.fields()
    );
    assert_eq!(
        matches,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_tombstone_import_appends_row_ids_to_fse_file() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("tombstone-base");
    let tombstone_csv_path = temp_csv_path("tombstone-row-ids");
    let archive_path = temp_archive_path("tombstone-row-ids", ".fse");
    let tombstone_path = temp_archive_path("tombstone-row-ids-tombstones", ".fse");
    let tombstone_csv = "\
entity_id,reason
103,withdrawn
";

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&tombstone_csv_path, tombstone_csv).unwrap();

    build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let result = append_typed_row_tombstone_archive_from_csv_file(
        &tombstone_csv_path,
        &tombstone_path,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
    )
    .unwrap();
    let loaded_tombstones = load_typed_row_tombstone_archive_file(&tombstone_path).unwrap();
    let tombstoned =
        load_typed_query_index_archive_with_tombstones(&archive_path, &tombstone_path).unwrap();
    let plan = score_and_class_plan(&schema, &mapping);

    assert_eq!(result.base_tombstone_count, 0);
    assert_eq!(result.requested_tombstone_count, 1);
    assert_eq!(result.appended_tombstone_count, 1);
    assert_eq!(result.resulting_tombstone_count, 1);
    assert_eq!(loaded_tombstones, vec![RowId::new(103)]);
    assert_eq!(
        tombstoned.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(tombstone_csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_tombstone_import_maintenance_compacts_fse_file() {
    let expected_schema = entity_schema();
    let mapping = FSESchemaDimensionMapping::identity(&expected_schema);
    let builder = builder();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 5_000).unwrap();
    let csv_path = temp_csv_path("tombstone-maintenance-base");
    let tombstone_csv_path = temp_csv_path("tombstone-maintenance-row-ids");
    let archive_path = temp_archive_path("tombstone-maintenance-row-ids", ".fse");
    let tombstone_path = temp_archive_path("tombstone-maintenance-row-ids-tombstones", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);
    let tombstone_csv = "\
entity_id,reason
103,withdrawn
";

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&tombstone_csv_path, tombstone_csv).unwrap();

    build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let result = maintain_typed_query_index_archive_from_csv_tombstone_file(
        &tombstone_csv_path,
        &archive_path,
        &tombstone_path,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
        &policy,
    )
    .unwrap();
    let loaded = load_typed_query_index_archive_file_with_encoder_metadata(&archive_path).unwrap();
    let loaded_tombstones = load_typed_row_tombstone_archive_file(&tombstone_path).unwrap();
    let plan = TypedQueryPlanBuilder::new(&expected_schema, &mapping)
        .try_with_record_encoder_metadata(&loaded.record_encoder_metadata)
        .unwrap()
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
        .unwrap();

    assert_eq!(result.tombstone_append.base_tombstone_count, 0);
    assert_eq!(result.tombstone_append.requested_tombstone_count, 1);
    assert_eq!(result.tombstone_append.appended_tombstone_count, 1);
    assert_eq!(result.tombstone_append.resulting_tombstone_count, 1);
    assert_eq!(
        result.maintenance.decision.action,
        FSEArchiveMaintenanceAction::Compact
    );
    assert_eq!(
        result.maintenance.decision.reason,
        FSEArchiveMaintenanceReason::CompactionTombstoneCountThresholdReached
    );
    assert_eq!(result.maintenance.decision.input.base_record_count, 4);
    assert_eq!(
        result
            .maintenance
            .decision
            .input
            .pending_append_record_count,
        0
    );
    assert_eq!(result.maintenance.decision.input.tombstone_count, 1);
    assert_eq!(result.maintenance.append_result, None);
    assert_eq!(
        result
            .maintenance
            .compaction_result
            .as_ref()
            .unwrap()
            .cleared_tombstone_count,
        1
    );
    assert_eq!(result.maintenance.query_index, loaded.query_index);
    assert_eq!(loaded.query_index.batch().len(), 3);
    assert_eq!(loaded_tombstones, Vec::new());
    assert_eq!(
        loaded.query_index.query_row_ids(&plan).unwrap(),
        vec![RowId::new(100)]
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(tombstone_csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_tombstone_import_reports_duplicate_row_ids() {
    let tombstone_csv_path = temp_csv_path("tombstone-duplicate-row-ids");
    let tombstone_path = temp_archive_path("tombstone-duplicate-row-ids", ".fse");
    let tombstone_csv = "\
entity_id,reason
103,withdrawn
103,duplicate
";

    fs::write(&tombstone_csv_path, tombstone_csv).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let error = append_typed_row_tombstone_archive_from_csv_file(
        &tombstone_csv_path,
        &tombstone_path,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
    )
    .unwrap_err();

    match error {
        FSECsvTombstoneImportError::Tombstones(
            FSETypedRowTombstoneArchiveError::DuplicateAppendedRowId { row_id },
        ) => assert_eq!(row_id, RowId::new(103)),
        other => panic!("expected duplicate tombstone row-id error, got {other}"),
    }
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(tombstone_csv_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_tombstone_import_maintenance_reports_duplicate_row_ids_and_preserves_archives() {
    let builder = builder();
    let csv_path = temp_csv_path("tombstone-maintenance-duplicate-base");
    let tombstone_csv_path = temp_csv_path("tombstone-maintenance-duplicate-row-ids");
    let archive_path = temp_archive_path("tombstone-maintenance-duplicate-row-ids", ".fse");
    let tombstone_path =
        temp_archive_path("tombstone-maintenance-duplicate-row-ids-tombstones", ".fse");
    let schema_options = FSECsvSchemaInferenceOptions::new()
        .with_field_type("class", FSEFieldType::Category)
        .with_field_type("observed_at", FSEFieldType::TimestampMillis);
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 1, 5_000).unwrap();
    let tombstone_csv = "\
entity_id,reason
103,withdrawn
103,duplicate
";

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&tombstone_csv_path, tombstone_csv).unwrap();

    let base = build_typed_query_index_archive_from_inferred_csv_file(
        &csv_path,
        &archive_path,
        &schema_options,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let error = maintain_typed_query_index_archive_from_csv_tombstone_file(
        &tombstone_csv_path,
        &archive_path,
        &tombstone_path,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &builder,
        &policy,
    )
    .unwrap_err();

    match error {
        FSECsvTombstoneMaintenanceImportError::Tombstones(
            FSETypedRowTombstoneArchiveError::DuplicateAppendedRowId { row_id },
        ) => assert_eq!(row_id, RowId::new(103)),
        other => panic!("expected duplicate tombstone row-id error, got {other}"),
    }
    assert_eq!(
        load_typed_query_index_archive_file(&archive_path).unwrap(),
        base.query_index
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(tombstone_csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_archive_import_reports_csv_read_error() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("missing");
    let archive_path = temp_archive_path("missing", ".fse");
    let _ = fs::remove_file(&csv_path);

    let error = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap_err();

    match error {
        FSECsvArchiveImportError::Csv(FSECsvFileImportError::Read { path, source }) => {
            assert_eq!(path, csv_path);
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        }
        other => panic!("expected CSV read error, got {other}"),
    }
    assert!(!archive_path.exists());
}

#[test]
fn csv_archive_import_reports_csv_parse_error() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("parse-error");
    let archive_path = temp_archive_path("parse-error", ".fse");
    let csv = "\
entity_id,score,class,observed_at
100,north,alpha,1000
";

    fs::write(&csv_path, csv).unwrap();

    let error = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap_err();

    match error {
        FSECsvArchiveImportError::Csv(FSECsvFileImportError::Import(
            FSECsvImportError::InvalidValue {
                line,
                field,
                expected,
                value,
            },
        )) => {
            assert_eq!(line, 2);
            assert_eq!(field, "score");
            assert_eq!(expected, FSEFieldType::Float);
            assert_eq!(value, "north");
        }
        other => panic!("expected CSV parse error, got {other}"),
    }
    assert!(!archive_path.exists());

    let _ = fs::remove_file(csv_path);
}

#[test]
fn csv_archive_import_append_reports_csv_parse_error_and_preserves_archive() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("append-parse-base");
    let append_csv_path = temp_csv_path("append-parse-error");
    let archive_path = temp_archive_path("append-parse-error", ".fse");
    let csv = "\
entity_id,score,class,observed_at
104,north,alpha,1400
";

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, csv).unwrap();

    let base_query_index = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();

    let error = append_typed_query_index_archive_from_csv_file(
        &append_csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap_err();

    match error {
        FSECsvArchiveImportError::Csv(FSECsvFileImportError::Import(
            FSECsvImportError::InvalidValue {
                line,
                field,
                expected,
                value,
            },
        )) => {
            assert_eq!(line, 2);
            assert_eq!(field, "score");
            assert_eq!(expected, FSEFieldType::Float);
            assert_eq!(value, "north");
        }
        other => panic!("expected CSV parse error, got {other}"),
    }
    assert_eq!(
        load_typed_query_index_archive_file(&archive_path).unwrap(),
        base_query_index
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_import_append_reports_archive_error_and_preserves_archive() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("append-archive-base");
    let append_csv_path = temp_csv_path("append-archive-error");
    let archive_path = temp_archive_path("append-archive-error", ".fse");
    let csv = "\
entity_id,score,class,observed_at
100,16.0,alpha,1400
";

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, csv).unwrap();

    let base_query_index = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();

    let error = append_typed_query_index_archive_from_csv_file(
        &append_csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap_err();

    match error {
        FSECsvArchiveImportError::Archive(FSETypedQueryIndexArchiveError::Append(
            TypedQueryIndexAppendError::RecordBatch(FSERecordBatchError::DuplicateRowId { row_id }),
        )) => assert_eq!(row_id, RowId::new(100)),
        other => panic!("expected archive append error, got {other}"),
    }
    assert_eq!(
        load_typed_query_index_archive_file(&archive_path).unwrap(),
        base_query_index
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
}

#[test]
fn csv_archive_import_maintenance_reports_csv_parse_error_and_preserves_archives() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let policy = FSEArchiveMaintenancePolicy::try_new(10, 10, 5_000).unwrap();
    let csv_path = temp_csv_path("maintenance-parse-base");
    let append_csv_path = temp_csv_path("maintenance-parse-error");
    let archive_path = temp_archive_path("maintenance-parse-error", ".fse");
    let tombstone_path = temp_archive_path("maintenance-parse-tombstones", ".fse");
    let csv = "\
entity_id,score,class,observed_at
104,north,alpha,1400
";

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, csv).unwrap();

    let base_query_index = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let error = maintain_typed_query_index_archive_from_csv_file(
        &append_csv_path,
        &archive_path,
        &tombstone_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
        &policy,
    )
    .unwrap_err();

    match error {
        FSECsvArchiveMaintenanceImportError::Csv(FSECsvFileImportError::Import(
            FSECsvImportError::InvalidValue {
                line,
                field,
                expected,
                value,
            },
        )) => {
            assert_eq!(line, 2);
            assert_eq!(field, "score");
            assert_eq!(expected, FSEFieldType::Float);
            assert_eq!(value, "north");
        }
        other => panic!("expected CSV parse error, got {other}"),
    }
    assert_eq!(
        load_typed_query_index_archive_file(&archive_path).unwrap(),
        base_query_index
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_archive_import_maintenance_reports_policy_error_and_preserves_archives() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let policy = FSEArchiveMaintenancePolicy {
        append_rebuild_record_count_threshold: 0,
        compaction_tombstone_count_threshold: 10,
        compaction_tombstone_ratio_threshold_basis_points: 5_000,
    };
    let csv_path = temp_csv_path("maintenance-policy-base");
    let append_csv_path = temp_csv_path("maintenance-policy-error");
    let archive_path = temp_archive_path("maintenance-policy-error", ".fse");
    let tombstone_path = temp_archive_path("maintenance-policy-tombstones", ".fse");

    fs::write(&csv_path, entity_csv()).unwrap();
    fs::write(&append_csv_path, appended_entity_csv()).unwrap();

    let base_query_index = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[]).unwrap();

    let error = maintain_typed_query_index_archive_from_csv_file(
        &append_csv_path,
        &archive_path,
        &tombstone_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
        &policy,
    )
    .unwrap_err();

    match error {
        FSECsvArchiveMaintenanceImportError::Maintenance(
            FSETypedQueryIndexArchiveMaintenanceError::Policy(
                FSEArchiveMaintenanceError::ZeroAppendRebuildRecordCountThreshold,
            ),
        ) => {}
        other => panic!("expected maintenance policy error, got {other}"),
    }
    assert_eq!(
        load_typed_query_index_archive_file(&archive_path).unwrap(),
        base_query_index
    );
    assert_eq!(
        load_typed_row_tombstone_archive_file(&tombstone_path).unwrap(),
        Vec::new()
    );

    let _ = fs::remove_file(csv_path);
    let _ = fs::remove_file(append_csv_path);
    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn csv_archive_import_reports_archive_error() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("archive-error");
    let archive_path = temp_archive_path("archive-error", ".bin");

    fs::write(&csv_path, entity_csv()).unwrap();

    let error = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        FSECsvArchiveImportError::Archive(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::InvalidFileExtension { .. }
        ))
    ));
    assert!(!archive_path.exists());

    let _ = fs::remove_file(csv_path);
}

#[test]
fn csv_archive_import_reports_write_error() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let csv_path = temp_csv_path("write-error");
    let archive_path = std::env::temp_dir()
        .join(format!(
            "fse-rust-csv-archive-import-{}-missing-dir",
            std::process::id()
        ))
        .join("archive.fse");

    fs::write(&csv_path, entity_csv()).unwrap();

    let error = build_typed_query_index_archive_from_csv_file(
        &csv_path,
        &archive_path,
        &schema,
        &FSECsvImportOptions::new().with_row_id_column("entity_id"),
        &encoder,
        &builder,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        FSECsvArchiveImportError::Archive(FSETypedQueryIndexArchiveError::File(
            FSETypedQueryIndexArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Write,
                ..
            }
        ))
    ));
    assert!(!archive_path.exists());

    let _ = fs::remove_file(csv_path);
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

fn score_and_class_predicates() -> Vec<FSEPredicate> {
    vec![
        FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(10.0),
            FSEValue::Float(20.0),
        ),
        FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category("alpha".to_string()),
        ),
    ]
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

fn class_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["alpha".to_string(), "beta".to_string()])
}

fn builder() -> FSEBuilder {
    FSEBuilder::new(BuildConfig::new(2, 8))
}

fn entity_csv() -> &'static str {
    "\
entity_id,score,class,observed_at
100,12.5,alpha,1000
101,12.5,beta,1100
102,25.0,alpha,1200
103,18.0,alpha,1300
"
}

fn appended_entity_csv() -> &'static str {
    "\
entity_id,score,class,observed_at
104,16.0,alpha,1400
105,80.0,beta,1500
"
}

fn temp_csv_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-csv-archive-import-{}-{}.csv",
        std::process::id(),
        name
    ));
    let _ = fs::remove_file(&path);
    path
}

fn temp_archive_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "fse-rust-csv-archive-import-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}
