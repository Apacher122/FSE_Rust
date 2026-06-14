use std::fs;
use std::io;
use std::path::PathBuf;

use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSECsvFileImportError, FSECsvImportError, FSECsvImportOptions, FSEDimensionMapping, FSEField,
    FSEFieldType, FSESchema, FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::import::{FSECsvArchiveImportError, build_typed_query_index_archive_from_csv_file};
use crate::persistence::{
    FSEArchiveFileOperation, FSETypedQueryIndexArchiveError, FSETypedQueryIndexArchiveFileError,
    load_typed_query_index_archive_file,
};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryPlanBuilder};

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
