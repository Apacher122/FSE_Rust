//! CSV import for FSE-native record batches.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError, FSERecordError,
    FSESchema, FSESchemaError, FSEValue, RowId,
};

/// Error returned when CSV import fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSECsvImportError {
    /// The CSV input did not contain any records.
    EmptyInput,

    /// A quoted CSV field was not closed.
    UnterminatedQuotedField {
        /// Line where the record began.
        line: usize,
    },

    /// A CSV row did not have the same width as the header row.
    RowWidthMismatch {
        /// Line containing the row.
        line: usize,
        /// Number of columns in the header.
        expected: usize,
        /// Number of columns in the row.
        actual: usize,
    },

    /// A schema field was not present in the CSV header.
    MissingSchemaField {
        /// Missing field name.
        field: String,
    },

    /// The configured row-id column was not present in the CSV header.
    MissingRowIdColumn {
        /// Missing row-id column name.
        column: String,
    },

    /// A CSV row-id value could not be parsed.
    InvalidRowId {
        /// Line containing the invalid row id.
        line: usize,
        /// Row-id column name.
        column: String,
        /// Invalid row-id value.
        value: String,
    },

    /// Generated row identifiers exceeded `u64`.
    GeneratedRowIdOverflow {
        /// Line where overflow was detected.
        line: usize,
    },

    /// A CSV field value could not be parsed as the schema field type.
    InvalidValue {
        /// Line containing the invalid value.
        line: usize,
        /// Schema field name.
        field: String,
        /// Expected field type.
        expected: FSEFieldType,
        /// Invalid CSV value.
        value: String,
    },

    /// A parsed record did not satisfy the schema.
    Record(FSERecordError),

    /// The resulting record batch was invalid.
    Batch(FSERecordBatchError),

    /// The inferred schema was invalid.
    Schema(FSESchemaError),
}

impl fmt::Display for FSECsvImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("CSV input must contain a header row"),
            Self::UnterminatedQuotedField { line } => {
                write!(
                    formatter,
                    "CSV quoted field starting on line {line} was not closed"
                )
            }
            Self::RowWidthMismatch {
                line,
                expected,
                actual,
            } => write!(
                formatter,
                "CSV row {line} has {actual} columns but header has {expected}"
            ),
            Self::MissingSchemaField { field } => {
                write!(formatter, "CSV header is missing schema field '{field}'")
            }
            Self::MissingRowIdColumn { column } => {
                write!(formatter, "CSV header is missing row-id column '{column}'")
            }
            Self::InvalidRowId {
                line,
                column,
                value,
            } => write!(
                formatter,
                "CSV row {line} has invalid row id '{value}' in column '{column}'"
            ),
            Self::GeneratedRowIdOverflow { line } => {
                write!(formatter, "CSV generated row id overflowed at line {line}")
            }
            Self::InvalidValue {
                line,
                field,
                expected,
                value,
            } => write!(
                formatter,
                "CSV row {line} field '{field}' expected {expected:?} but found '{value}'"
            ),
            Self::Record(error) => error.fmt(formatter),
            Self::Batch(error) => error.fmt(formatter),
            Self::Schema(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Record(error) => Some(error),
            Self::Batch(error) => Some(error),
            Self::Schema(error) => Some(error),
            Self::EmptyInput
            | Self::UnterminatedQuotedField { .. }
            | Self::RowWidthMismatch { .. }
            | Self::MissingSchemaField { .. }
            | Self::MissingRowIdColumn { .. }
            | Self::InvalidRowId { .. }
            | Self::GeneratedRowIdOverflow { .. }
            | Self::InvalidValue { .. } => None,
        }
    }
}

impl From<FSERecordError> for FSECsvImportError {
    fn from(error: FSERecordError) -> Self {
        Self::Record(error)
    }
}

impl From<FSERecordBatchError> for FSECsvImportError {
    fn from(error: FSERecordBatchError) -> Self {
        Self::Batch(error)
    }
}

impl From<FSESchemaError> for FSECsvImportError {
    fn from(error: FSESchemaError) -> Self {
        Self::Schema(error)
    }
}

/// Error returned when CSV file import fails.
#[derive(Debug)]
pub enum FSECsvFileImportError {
    /// The CSV file could not be read as UTF-8 text.
    Read {
        /// Path passed to the importer.
        path: PathBuf,
        /// Source I/O error.
        source: io::Error,
    },

    /// The CSV text failed record-batch import validation.
    Import(FSECsvImportError),
}

impl fmt::Display for FSECsvFileImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read CSV file '{}': {source}",
                    path.display()
                )
            }
            Self::Import(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSECsvFileImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Import(error) => Some(error),
        }
    }
}

impl From<FSECsvImportError> for FSECsvFileImportError {
    fn from(error: FSECsvImportError) -> Self {
        Self::Import(error)
    }
}

/// CSV import options for typed record batches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FSECsvImportOptions {
    row_id_column: Option<String>,
    generated_row_id_start: u64,
}

impl Default for FSECsvImportOptions {
    fn default() -> Self {
        Self {
            row_id_column: None,
            generated_row_id_start: 1,
        }
    }
}

impl FSECsvImportOptions {
    /// Creates CSV import options with generated row identifiers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Uses a CSV column as the row identifier source.
    pub fn with_row_id_column(mut self, column: impl Into<String>) -> Self {
        self.row_id_column = Some(column.into());
        self
    }

    /// Sets the first generated row identifier.
    pub fn with_generated_row_id_start(mut self, start: u64) -> Self {
        self.generated_row_id_start = start;
        self
    }

    /// Returns the configured row-id column name.
    pub fn row_id_column(&self) -> Option<&str> {
        self.row_id_column.as_deref()
    }

    /// Returns the first generated row identifier.
    pub fn generated_row_id_start(&self) -> u64 {
        self.generated_row_id_start
    }
}

/// Imports a typed record batch from CSV text.
///
/// The first CSV record is treated as the header. Data columns are matched to
/// schema fields by name. Row identifiers are either generated or read from the
/// configured row-id column.
pub fn record_batch_from_csv(
    input: &str,
    schema: &FSESchema,
    options: &FSECsvImportOptions,
) -> Result<FSERecordBatch, FSECsvImportError> {
    let records = parse_csv_records(input)?;
    let (header, data_records) = records.split_first().ok_or(FSECsvImportError::EmptyInput)?;
    let header_index = header_index(header);
    let field_indexes = schema_field_indexes(schema, &header_index)?;
    let row_id_index = row_id_index(options, &header_index)?;
    let mut row_ids = Vec::with_capacity(data_records.len());
    let mut typed_records = Vec::with_capacity(data_records.len());

    for (record_offset, csv_record) in data_records.iter().enumerate() {
        if csv_record.fields.len() != header.fields.len() {
            return Err(FSECsvImportError::RowWidthMismatch {
                line: csv_record.line,
                expected: header.fields.len(),
                actual: csv_record.fields.len(),
            });
        }

        let row_id = csv_row_id(csv_record, row_id_index, options, record_offset)?;
        let values = csv_record_values(csv_record, schema, &field_indexes)?;
        let record = FSERecord::try_new(values, schema)?;

        row_ids.push(row_id);
        typed_records.push(record);
    }

    Ok(FSERecordBatch::try_new(
        schema.clone(),
        row_ids,
        typed_records,
    )?)
}

/// Infers schema metadata from CSV text.
///
/// The first CSV record is treated as the header. Field types are inferred from
/// non-empty values in each column. Empty values mark the field as nullable.
pub fn infer_schema_from_csv(input: &str) -> Result<FSESchema, FSECsvImportError> {
    let records = parse_csv_records(input)?;
    let (header, data_records) = records.split_first().ok_or(FSECsvImportError::EmptyInput)?;
    let mut columns = vec![CsvSchemaColumnInference::default(); header.fields.len()];

    for csv_record in data_records {
        if csv_record.fields.len() != header.fields.len() {
            return Err(FSECsvImportError::RowWidthMismatch {
                line: csv_record.line,
                expected: header.fields.len(),
                actual: csv_record.fields.len(),
            });
        }

        for (column, value) in columns.iter_mut().zip(&csv_record.fields) {
            column.observe(value);
        }
    }

    let fields = header
        .fields
        .iter()
        .zip(columns)
        .map(|(name, column)| {
            FSEField::new(
                name.trim().to_string(),
                column.inferred_field_type(),
                column.nullable,
            )
        })
        .collect();

    Ok(FSESchema::try_new(fields)?)
}

/// Imports a typed record batch from a UTF-8 CSV file.
///
/// The file contents are parsed with [`record_batch_from_csv`].
pub fn record_batch_from_csv_file(
    path: impl AsRef<Path>,
    schema: &FSESchema,
    options: &FSECsvImportOptions,
) -> Result<FSERecordBatch, FSECsvFileImportError> {
    let path = path.as_ref();
    let input = fs::read_to_string(path).map_err(|source| FSECsvFileImportError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(record_batch_from_csv(&input, schema, options)?)
}

/// Infers schema metadata from a UTF-8 CSV file.
///
/// The file contents are parsed with [`infer_schema_from_csv`].
pub fn infer_schema_from_csv_file(
    path: impl AsRef<Path>,
) -> Result<FSESchema, FSECsvFileImportError> {
    let path = path.as_ref();
    let input = fs::read_to_string(path).map_err(|source| FSECsvFileImportError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(infer_schema_from_csv(&input)?)
}

#[derive(Clone, Debug)]
struct CsvSchemaColumnInference {
    nullable: bool,
    saw_non_null_value: bool,
    can_be_boolean: bool,
    can_be_integer: bool,
    can_be_float: bool,
}

impl Default for CsvSchemaColumnInference {
    fn default() -> Self {
        Self {
            nullable: false,
            saw_non_null_value: false,
            can_be_boolean: true,
            can_be_integer: true,
            can_be_float: true,
        }
    }
}

impl CsvSchemaColumnInference {
    fn observe(&mut self, raw_value: &str) {
        let value = raw_value.trim();

        if value.is_empty() {
            self.nullable = true;
            return;
        }

        self.saw_non_null_value = true;
        self.can_be_boolean &= parse_boolean(value).is_some();
        self.can_be_integer &= value.parse::<i64>().is_ok();
        self.can_be_float &= value
            .parse::<f64>()
            .map(|value| value.is_finite())
            .unwrap_or(false);
    }

    fn inferred_field_type(&self) -> FSEFieldType {
        if !self.saw_non_null_value {
            return FSEFieldType::Text;
        }

        if self.can_be_boolean {
            FSEFieldType::Boolean
        } else if self.can_be_integer {
            FSEFieldType::Integer
        } else if self.can_be_float {
            FSEFieldType::Float
        } else {
            FSEFieldType::Text
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CsvRecord {
    line: usize,
    fields: Vec<String>,
}

fn parse_csv_records(input: &str) -> Result<Vec<CsvRecord>, FSECsvImportError> {
    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut characters = input.chars().peekable();
    let mut line = 1;
    let mut record_line = 1;
    let mut in_quotes = false;
    let mut saw_input = false;

    while let Some(character) = characters.next() {
        saw_input = true;

        match character {
            '"' if in_quotes && characters.peek() == Some(&'"') => {
                field.push('"');
                characters.next();
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut field));
            }
            '\n' if !in_quotes => {
                push_csv_record(&mut records, record_line, &mut fields, &mut field);
                line += 1;
                record_line = line;
            }
            '\r' if !in_quotes => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                push_csv_record(&mut records, record_line, &mut fields, &mut field);
                line += 1;
                record_line = line;
            }
            '\n' => {
                line += 1;
                field.push(character);
            }
            _ => field.push(character),
        }
    }

    if in_quotes {
        return Err(FSECsvImportError::UnterminatedQuotedField { line: record_line });
    }

    if saw_input && (!field.is_empty() || !fields.is_empty()) {
        push_csv_record(&mut records, record_line, &mut fields, &mut field);
    }

    if records.is_empty() {
        return Err(FSECsvImportError::EmptyInput);
    }

    Ok(records)
}

fn push_csv_record(
    records: &mut Vec<CsvRecord>,
    line: usize,
    fields: &mut Vec<String>,
    field: &mut String,
) {
    fields.push(std::mem::take(field));

    if fields.len() == 1 && fields[0].trim().is_empty() {
        fields.clear();
        return;
    }

    records.push(CsvRecord {
        line,
        fields: std::mem::take(fields),
    });
}

fn header_index(header: &CsvRecord) -> HashMap<String, usize> {
    header
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| (field.trim().to_string(), index))
        .collect()
}

fn schema_field_indexes(
    schema: &FSESchema,
    header_index: &HashMap<String, usize>,
) -> Result<Vec<usize>, FSECsvImportError> {
    schema
        .fields()
        .iter()
        .map(|field| {
            header_index.get(&field.name).copied().ok_or_else(|| {
                FSECsvImportError::MissingSchemaField {
                    field: field.name.clone(),
                }
            })
        })
        .collect()
}

fn row_id_index(
    options: &FSECsvImportOptions,
    header_index: &HashMap<String, usize>,
) -> Result<Option<usize>, FSECsvImportError> {
    match options.row_id_column() {
        Some(column) => header_index.get(column).copied().map(Some).ok_or_else(|| {
            FSECsvImportError::MissingRowIdColumn {
                column: column.to_string(),
            }
        }),
        None => Ok(None),
    }
}

fn csv_row_id(
    record: &CsvRecord,
    row_id_index: Option<usize>,
    options: &FSECsvImportOptions,
    record_offset: usize,
) -> Result<RowId, FSECsvImportError> {
    if let Some(row_id_index) = row_id_index {
        let value = record.fields[row_id_index].trim();
        let row_id = value
            .parse::<u64>()
            .map_err(|_| FSECsvImportError::InvalidRowId {
                line: record.line,
                column: options
                    .row_id_column()
                    .expect("row-id index requires configured column")
                    .to_string(),
                value: value.to_string(),
            })?;

        return Ok(RowId::new(row_id));
    }

    let row_id = options
        .generated_row_id_start()
        .checked_add(record_offset as u64)
        .ok_or(FSECsvImportError::GeneratedRowIdOverflow { line: record.line })?;

    Ok(RowId::new(row_id))
}

fn csv_record_values(
    record: &CsvRecord,
    schema: &FSESchema,
    field_indexes: &[usize],
) -> Result<Vec<FSEValue>, FSECsvImportError> {
    schema
        .fields()
        .iter()
        .zip(field_indexes)
        .map(|(field, index)| {
            csv_value(
                record,
                &field.name,
                field.field_type,
                &record.fields[*index],
            )
        })
        .collect()
}

fn csv_value(
    record: &CsvRecord,
    field_name: &str,
    field_type: FSEFieldType,
    raw_value: &str,
) -> Result<FSEValue, FSECsvImportError> {
    let value = raw_value.trim();

    if value.is_empty() {
        return Ok(FSEValue::Null);
    }

    match field_type {
        FSEFieldType::Integer => value
            .parse::<i64>()
            .map(FSEValue::Integer)
            .map_err(|_| invalid_value(record, field_name, field_type, value)),
        FSEFieldType::Float => {
            let parsed = value
                .parse::<f64>()
                .map_err(|_| invalid_value(record, field_name, field_type, value))?;

            if !parsed.is_finite() {
                return Err(invalid_value(record, field_name, field_type, value));
            }

            Ok(FSEValue::Float(parsed))
        }
        FSEFieldType::Text => Ok(FSEValue::Text(value.to_string())),
        FSEFieldType::Boolean => parse_boolean(value)
            .map(FSEValue::Boolean)
            .ok_or_else(|| invalid_value(record, field_name, field_type, value)),
        FSEFieldType::TimestampMillis => value
            .parse::<i64>()
            .map(FSEValue::TimestampMillis)
            .map_err(|_| invalid_value(record, field_name, field_type, value)),
        FSEFieldType::Category => Ok(FSEValue::Category(value.to_string())),
    }
}

fn parse_boolean(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn invalid_value(
    record: &CsvRecord,
    field_name: &str,
    field_type: FSEFieldType,
    value: &str,
) -> FSECsvImportError {
    FSECsvImportError::InvalidValue {
        line: record.line,
        field: field_name.to_string(),
        expected: field_type,
        value: value.to_string(),
    }
}
