use std::fs;
use std::io;
use std::path::PathBuf;

use crate::data::{
    FSECsvFileImportError, FSECsvImportError, FSECsvImportOptions, FSEField, FSEFieldType,
    FSERecordError, FSESchema, FSEValue, RowId, record_batch_from_csv, record_batch_from_csv_file,
};

#[test]
fn csv_import_builds_record_batch_with_generated_row_ids() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,burglary,41.881,true,1735689600000,open
43,theft,41.882,false,1735689700000,closed
";

    let batch = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect("valid CSV should import");

    assert_eq!(batch.len(), 2);
    assert_eq!(batch.row_ids(), &[RowId::new(1), RowId::new(2)]);
    assert_eq!(
        batch.records()[0].value_named(&schema, "category"),
        Some(&FSEValue::Text("burglary".to_string()))
    );
    assert_eq!(
        batch.records()[0].value_named(&schema, "closed"),
        Some(&FSEValue::Boolean(true))
    );
    assert_eq!(
        batch.records()[1].value_named(&schema, "status"),
        Some(&FSEValue::Category("closed".to_string()))
    );
}

#[test]
fn csv_import_uses_row_id_column() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,burglary,41.881,true,1735689600000,open
";
    let options = FSECsvImportOptions::new().with_row_id_column("case_id");

    let batch = record_batch_from_csv(csv, &schema, &options).expect("row-id CSV should import");

    assert_eq!(batch.row_ids(), &[RowId::new(42)]);
    assert_eq!(
        batch.records()[0].value_named(&schema, "case_id"),
        Some(&FSEValue::Integer(42))
    );
}

#[test]
fn csv_import_parses_quoted_commas_and_escaped_quotes() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,\"burglary, forced entry\",41.881,true,1735689600000,\"open \"\"review\"\"\"
";

    let batch = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect("quoted CSV should import");

    assert_eq!(
        batch.records()[0].value_named(&schema, "category"),
        Some(&FSEValue::Text("burglary, forced entry".to_string()))
    );
    assert_eq!(
        batch.records()[0].value_named(&schema, "status"),
        Some(&FSEValue::Category("open \"review\"".to_string()))
    );
}

#[test]
fn csv_import_allows_nullable_empty_values() {
    let schema = FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("notes", FSEFieldType::Text, true),
    ]);
    let csv = "\
case_id,notes
42,
";

    let batch = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect("nullable CSV value should import");

    assert_eq!(
        batch.records()[0].value_named(&schema, "notes"),
        Some(&FSEValue::Null)
    );
}

#[test]
fn csv_import_reports_missing_schema_field() {
    let schema = event_schema();
    let csv = "\
case_id,category,closed,created_at,status
42,burglary,true,1735689600000,open
";

    let error = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect_err("missing schema field should be rejected");

    assert_eq!(
        error,
        FSECsvImportError::MissingSchemaField {
            field: "latitude".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "CSV header is missing schema field 'latitude'"
    );
}

#[test]
fn csv_import_reports_row_width_mismatch() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,burglary,41.881,true,1735689600000
";

    let error = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect_err("row width mismatch should be rejected");

    assert_eq!(
        error,
        FSECsvImportError::RowWidthMismatch {
            line: 2,
            expected: 6,
            actual: 5,
        }
    );
}

#[test]
fn csv_import_reports_invalid_field_value() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,burglary,north,true,1735689600000,open
";

    let error = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect_err("invalid typed value should be rejected");

    assert_eq!(
        error,
        FSECsvImportError::InvalidValue {
            line: 2,
            field: "latitude".to_string(),
            expected: FSEFieldType::Float,
            value: "north".to_string(),
        }
    );
}

#[test]
fn csv_import_reports_null_for_non_nullable_field() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,,41.881,true,1735689600000,open
";

    let error = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect_err("null in non-nullable field should be rejected");

    assert_eq!(
        error,
        FSECsvImportError::Record(FSERecordError::NullNotAllowed {
            field: 1,
            name: "category".to_string(),
        })
    );
}

#[test]
fn csv_import_reports_invalid_row_id() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
not-a-row-id,burglary,41.881,true,1735689600000,open
";
    let options = FSECsvImportOptions::new().with_row_id_column("case_id");

    let error = record_batch_from_csv(csv, &schema, &options)
        .expect_err("invalid row id should be rejected");

    assert_eq!(
        error,
        FSECsvImportError::InvalidRowId {
            line: 2,
            column: "case_id".to_string(),
            value: "not-a-row-id".to_string(),
        }
    );
}

#[test]
fn csv_import_reports_unclosed_quote() {
    let schema = event_schema();
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,\"burglary,41.881,true,1735689600000,open
";

    let error = record_batch_from_csv(csv, &schema, &FSECsvImportOptions::new())
        .expect_err("unclosed quote should be rejected");

    assert_eq!(
        error,
        FSECsvImportError::UnterminatedQuotedField { line: 2 }
    );
}

#[test]
fn csv_file_import_reads_record_batch_from_path() {
    let schema = event_schema();
    let path = temp_csv_path("csv_file_import_reads_record_batch_from_path");
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,burglary,41.881,true,1735689600000,open
";
    fs::write(&path, csv).expect("test CSV file should be written");

    let batch = record_batch_from_csv_file(&path, &schema, &FSECsvImportOptions::new())
        .expect("CSV file should import");

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.row_ids(), &[RowId::new(1)]);
    assert_eq!(
        batch.records()[0].value_named(&schema, "category"),
        Some(&FSEValue::Text("burglary".to_string()))
    );

    fs::remove_file(path).expect("test CSV file should be removed");
}

#[test]
fn csv_file_import_reports_read_error() {
    let schema = event_schema();
    let path = temp_csv_path("csv_file_import_reports_read_error");
    let _ = fs::remove_file(&path);

    let error = record_batch_from_csv_file(&path, &schema, &FSECsvImportOptions::new())
        .expect_err("missing CSV file should be rejected");

    match error {
        FSECsvFileImportError::Read {
            path: error_path,
            source,
        } => {
            assert_eq!(error_path, path);
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        }
        FSECsvFileImportError::Import(error) => {
            panic!("expected read error, got import error: {error}");
        }
    }
}

#[test]
fn csv_file_import_reports_import_error() {
    let schema = event_schema();
    let path = temp_csv_path("csv_file_import_reports_import_error");
    let csv = "\
case_id,category,latitude,closed,created_at,status
42,burglary,north,true,1735689600000,open
";
    fs::write(&path, csv).expect("test CSV file should be written");

    let error = record_batch_from_csv_file(&path, &schema, &FSECsvImportOptions::new())
        .expect_err("invalid CSV file value should be rejected");

    match error {
        FSECsvFileImportError::Import(FSECsvImportError::InvalidValue {
            line,
            field,
            expected,
            value,
        }) => {
            assert_eq!(line, 2);
            assert_eq!(field, "latitude");
            assert_eq!(expected, FSEFieldType::Float);
            assert_eq!(value, "north");
        }
        other => panic!("expected invalid field value import error, got {other}"),
    }

    fs::remove_file(path).expect("test CSV file should be removed");
}

fn event_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("category", FSEFieldType::Text, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("closed", FSEFieldType::Boolean, false),
        FSEField::new("created_at", FSEFieldType::TimestampMillis, false),
        FSEField::new("status", FSEFieldType::Category, false),
    ])
}

fn temp_csv_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}.csv", std::process::id()))
}
