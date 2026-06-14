//! FSE-native typed data.
//!
//! This module defines typed logical values and schema metadata used before
//! semantic encoding maps records into numeric coordinate space.

mod batch;
mod csv;
mod dataset;
mod mapping;
mod record;
mod row_id;
mod schema;
mod value;

pub use batch::{FSERecordBatch, FSERecordBatchError};
pub use csv::{
    FSECsvFileImportError, FSECsvImportError, FSECsvImportOptions, record_batch_from_csv,
    record_batch_from_csv_file,
};
pub use dataset::{FSEDatasetMetadata, FSEDatasetMetadataError};
pub use mapping::{FSEDimensionMapping, FSESchemaDimensionMapping, FSESchemaDimensionMappingError};
pub use record::{FSERecord, FSERecordError};
pub use row_id::RowId;
pub use schema::{FSEField, FSESchema, FSESchemaError};
pub use value::{FSEFieldType, FSEValue};
