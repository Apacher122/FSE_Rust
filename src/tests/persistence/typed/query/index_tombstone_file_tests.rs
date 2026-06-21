use std::fs;
use std::io;
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
    FSEArchiveFileOperation, FSETombstonedTypedQueryIndex,
    FSETombstonedTypedQueryIndexArchiveError, FSETypedQueryIndexArchiveError,
    FSETypedQueryIndexArchiveFileError, FSETypedRowTombstoneArchiveError,
    FSETypedRowTombstoneArchiveFileError, load_typed_query_index_archive_with_tombstones,
    save_typed_query_index_archive_file, save_typed_row_tombstone_archive_file,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryOutputContract,
    TypedQueryPlanBuilder, TypedRowTombstoneSet,
};

#[test]
fn typed_query_index_archive_with_tombstones_loads_and_filters_results() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let index_path = temp_archive_path("load-index", ".fse");
    let tombstone_path = temp_archive_path("load-tombstones", ".fse");

    save_typed_query_index_archive_file(&index_path, &query_index).unwrap();
    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(100)]).unwrap();

    let loaded =
        load_typed_query_index_archive_with_tombstones(&index_path, &tombstone_path).unwrap();
    let mut visited_row_ids = Vec::new();
    let mut visited_records = Vec::new();

    let row_id_report = loaded.query_row_ids_with_stats(&plan).unwrap();
    let row_report = loaded.query_rows_with_stats(&plan).unwrap();
    let count_report = loaded.count_matches_with_stats(&plan).unwrap();
    let existence_report = loaded.has_match_with_stats(&plan).unwrap();
    let planned_row_id_report = loaded.query_row_ids_with_planning(&plan).unwrap();
    let planned_row_report = loaded.query_rows_with_planning(&plan).unwrap();
    let planned_count_report = loaded.count_matches_with_planning(&plan).unwrap();
    let planned_existence_report = loaded.has_match_with_planning(&plan).unwrap();
    let row_id_stats = loaded
        .visit_row_ids(&plan, |row_id| {
            visited_row_ids.push(row_id);
        })
        .unwrap();
    let row_stats = loaded
        .visit_rows(&plan, |row_id, record| {
            visited_records.push((row_id, record.clone()));
        })
        .unwrap();
    let mut planned_visited_row_ids = Vec::new();
    let planned_row_id_visit_report = loaded
        .visit_row_ids_with_planning(&plan, |row_id| {
            planned_visited_row_ids.push(row_id);
        })
        .unwrap();
    let mut planned_visited_records = Vec::new();
    let planned_row_visit_report = loaded
        .visit_rows_with_planning(&plan, |row_id, record| {
            planned_visited_records.push((row_id, record.clone()));
        })
        .unwrap();

    assert_eq!(loaded.query_index(), &query_index);
    assert!(loaded.tombstones().contains(RowId::new(100)));
    assert_eq!(loaded.tombstones().row_ids(), &[RowId::new(100)]);
    assert_eq!(
        query_index.query_row_ids(&plan).unwrap(),
        matching_row_ids()
    );
    assert_eq!(loaded.query_row_ids(&plan).unwrap(), vec![RowId::new(103)]);
    assert_eq!(row_id_report.row_ids, vec![RowId::new(103)]);
    assert_eq!(row_id_report.stats.matched_records, 1);
    assert_eq!(planned_row_id_report.row_ids, vec![RowId::new(103)]);
    assert_eq!(
        planned_row_id_report.diagnostics.output_contract,
        TypedQueryOutputContract::RowIds
    );
    assert_eq!(
        loaded.query_rows(&plan).unwrap()[0].row_id(),
        RowId::new(103)
    );
    assert_eq!(row_report.rows.len(), 1);
    assert_eq!(row_report.rows[0].row_id(), RowId::new(103));
    assert_eq!(row_report.stats.matched_records, 1);
    assert_eq!(planned_row_report.rows.len(), 1);
    assert_eq!(planned_row_report.rows[0].row_id(), RowId::new(103));
    assert_eq!(
        planned_row_report.diagnostics.output_contract,
        TypedQueryOutputContract::Rows
    );
    assert_eq!(loaded.count_matches(&plan).unwrap(), 1);
    assert_eq!(count_report.matched_records, 1);
    assert_eq!(count_report.stats.matched_records, 1);
    assert_eq!(planned_count_report.matched_records, 1);
    assert_eq!(
        planned_count_report.diagnostics.output_contract,
        TypedQueryOutputContract::Count
    );
    assert!(loaded.has_match(&plan).unwrap());
    assert!(existence_report.has_match);
    assert_eq!(existence_report.stats.matched_records, 1);
    assert!(planned_existence_report.has_match);
    assert_eq!(
        planned_existence_report.diagnostics.output_contract,
        TypedQueryOutputContract::Existence
    );
    assert_eq!(visited_row_ids, vec![RowId::new(103)]);
    assert_eq!(row_id_stats.matched_records, 1);
    assert_eq!(planned_visited_row_ids, vec![RowId::new(103)]);
    assert_eq!(planned_row_id_visit_report.visited_records, 1);
    assert_eq!(
        planned_row_id_visit_report.diagnostics.output_contract,
        TypedQueryOutputContract::RowIdVisitor
    );
    assert_eq!(visited_records.len(), 1);
    assert_eq!(visited_records[0].0, RowId::new(103));
    assert_eq!(row_stats.matched_records, 1);
    assert_eq!(planned_visited_records.len(), 1);
    assert_eq!(planned_visited_records[0].0, RowId::new(103));
    assert_eq!(planned_row_visit_report.visited_records, 1);
    assert_eq!(
        planned_row_visit_report.diagnostics.output_contract,
        TypedQueryOutputContract::RowVisitor
    );

    let _ = fs::remove_file(index_path);
    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn tombstoned_typed_query_index_from_row_ids_filters_in_memory_index() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstoned =
        FSETombstonedTypedQueryIndex::from_row_ids(query_index, [RowId::new(100), RowId::new(103)]);

    assert_eq!(
        tombstoned.query_row_ids(&plan).unwrap(),
        Vec::<RowId>::new()
    );
    assert_eq!(tombstoned.count_matches(&plan).unwrap(), 0);
    assert!(!tombstoned.has_match(&plan).unwrap());
    assert_eq!(tombstoned.tombstones().len(), 2);
}

#[test]
fn tombstoned_typed_query_index_accepts_existing_tombstone_set() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let query_index = typed_query_index();
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(100)]);
    let tombstoned = FSETombstonedTypedQueryIndex::new(query_index, tombstones);

    assert_eq!(
        tombstoned.query_row_ids(&plan).unwrap(),
        vec![RowId::new(103)]
    );
}

#[test]
fn typed_query_index_archive_with_tombstones_reports_missing_index_file() {
    let index_path = temp_archive_path("missing-index", ".fse");
    let tombstone_path = temp_archive_path("valid-tombstones", ".fse");

    save_typed_row_tombstone_archive_file(&tombstone_path, &[RowId::new(100)]).unwrap();

    assert_eq!(
        load_typed_query_index_archive_with_tombstones(&index_path, &tombstone_path),
        Err(FSETombstonedTypedQueryIndexArchiveError::QueryIndex(
            FSETypedQueryIndexArchiveError::File(FSETypedQueryIndexArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Read,
                path: index_path,
                kind: io::ErrorKind::NotFound,
            })
        ))
    );

    let _ = fs::remove_file(tombstone_path);
}

#[test]
fn typed_query_index_archive_with_tombstones_reports_missing_tombstone_file() {
    let query_index = typed_query_index();
    let index_path = temp_archive_path("valid-index", ".fse");
    let tombstone_path = temp_archive_path("missing-tombstones", ".fse");

    save_typed_query_index_archive_file(&index_path, &query_index).unwrap();

    assert_eq!(
        load_typed_query_index_archive_with_tombstones(&index_path, &tombstone_path),
        Err(FSETombstonedTypedQueryIndexArchiveError::Tombstones(
            FSETypedRowTombstoneArchiveError::File(FSETypedRowTombstoneArchiveFileError::Io {
                operation: FSEArchiveFileOperation::Read,
                path: tombstone_path,
                kind: io::ErrorKind::NotFound,
            })
        ))
    );

    let _ = fs::remove_file(index_path);
}

fn typed_query_index() -> TypedQueryIndex {
    let schema = entity_schema();
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
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

fn matching_row_ids() -> Vec<RowId> {
    vec![RowId::new(100), RowId::new(103)]
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
        "fse-rust-typed-query-index-tombstone-file-{}-{}{}",
        std::process::id(),
        name,
        extension
    ));
    let _ = fs::remove_file(&path);
    path
}
