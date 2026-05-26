//! CSV document formatting helpers.

use crate::benchmark::formatting::{format_f64_fixed_6, format_scalar_fixed_6};
use crate::math::Scalar;

use super::metadata::{BenchmarkCsvMetadata, metadata_header_fields, metadata_value_fields};

pub(super) fn header_fields_with_metadata(header_fields: Vec<&'static str>) -> Vec<&'static str> {
    let mut fields = metadata_header_fields();
    fields.extend(header_fields);
    fields
}

pub(super) fn value_fields_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    value_fields: Vec<String>,
) -> Vec<String> {
    let mut fields = metadata_value_fields(metadata);
    fields.extend(value_fields);
    fields
}

pub(super) fn csv_document<I>(header_fields: Vec<&'static str>, value_rows: I) -> String
where
    I: IntoIterator<Item = Vec<String>>,
{
    let mut rows = Vec::new();

    rows.push(csv_row(header_fields));

    for value_row in value_rows {
        rows.push(csv_row(value_row));
    }

    rows.join("\n")
}

fn csv_row<I, S>(fields: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fields
        .into_iter()
        .map(|field| escape_csv_field(field.as_ref()))
        .collect::<Vec<String>>()
        .join(",")
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}

pub(super) fn format_scalar(value: Scalar) -> String {
    format_scalar_fixed_6(value)
}

pub(super) fn format_ratio(value: f64) -> String {
    format_f64_fixed_6(value)
}
