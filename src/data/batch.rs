//! FSE-native record batches.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use super::{FSERecord, FSESchema, RowId};

/// Error returned when checked record batch construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERecordBatchError {
    /// The row identifier count did not match the record count.
    RowIdCountMismatch {
        /// Number of row identifiers provided.
        row_id_count: usize,
        /// Number of records provided.
        record_count: usize,
    },

    /// A row identifier appeared more than once.
    DuplicateRowId {
        /// Repeated row identifier.
        row_id: RowId,
    },

    /// The appended batch used a different schema.
    SchemaMismatch,

    /// The appended batch contained no records.
    EmptyAppendBatch,
}

impl fmt::Display for FSERecordBatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowIdCountMismatch {
                row_id_count,
                record_count,
            } => write!(
                formatter,
                "record batch has {row_id_count} row ids but {record_count} records"
            ),
            Self::DuplicateRowId { row_id } => {
                write!(
                    formatter,
                    "row id {} appears more than once",
                    row_id.value()
                )
            }
            Self::SchemaMismatch => {
                formatter.write_str("record batch append requires matching schemas")
            }
            Self::EmptyAppendBatch => {
                formatter.write_str("record batch append requires at least one appended record")
            }
        }
    }
}

impl Error for FSERecordBatchError {}

/// Batch of typed records with stable row identifiers.
///
/// # Runtime Role
///
/// `FSERecordBatch` groups records that share one schema. Row identifiers are
/// stored beside records so encoded coordinates and query results can map back
/// to stable logical rows.
#[derive(Clone, Debug, PartialEq)]
pub struct FSERecordBatch {
    schema: FSESchema,
    row_ids: Vec<RowId>,
    records: Vec<FSERecord>,
    row_index_by_id: HashMap<RowId, usize>,
}

impl FSERecordBatch {
    /// Creates a record batch from schema, row identifiers, and records.
    ///
    /// # Panics
    ///
    /// Panics when the row identifier count does not match the record count or
    /// when row identifiers are duplicated.
    pub fn new(schema: FSESchema, row_ids: Vec<RowId>, records: Vec<FSERecord>) -> Self {
        Self::try_new(schema, row_ids, records).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a record batch and returns an error when metadata is invalid.
    pub fn try_new(
        schema: FSESchema,
        row_ids: Vec<RowId>,
        records: Vec<FSERecord>,
    ) -> Result<Self, FSERecordBatchError> {
        if row_ids.len() != records.len() {
            return Err(FSERecordBatchError::RowIdCountMismatch {
                row_id_count: row_ids.len(),
                record_count: records.len(),
            });
        }

        let mut row_index_by_id = HashMap::with_capacity(row_ids.len());

        for (index, row_id) in row_ids.iter().enumerate() {
            if row_index_by_id.insert(*row_id, index).is_some() {
                return Err(FSERecordBatchError::DuplicateRowId { row_id: *row_id });
            }
        }

        Ok(Self {
            schema,
            row_ids,
            records,
            row_index_by_id,
        })
    }

    /// Appends another batch and returns a checked combined batch.
    pub fn try_append(&self, appended: &Self) -> Result<Self, FSERecordBatchError> {
        if self.schema != appended.schema {
            return Err(FSERecordBatchError::SchemaMismatch);
        }

        if appended.is_empty() {
            return Err(FSERecordBatchError::EmptyAppendBatch);
        }

        let mut row_ids = Vec::with_capacity(self.row_ids.len() + appended.row_ids.len());
        row_ids.extend_from_slice(&self.row_ids);
        row_ids.extend_from_slice(&appended.row_ids);

        let mut records = Vec::with_capacity(self.records.len() + appended.records.len());
        records.extend_from_slice(&self.records);
        records.extend_from_slice(&appended.records);

        Self::try_new(self.schema.clone(), row_ids, records)
    }

    /// Returns the schema shared by the batch records.
    pub fn schema(&self) -> &FSESchema {
        &self.schema
    }

    /// Returns row identifiers in record order.
    pub fn row_ids(&self) -> &[RowId] {
        &self.row_ids
    }

    /// Returns records in row identifier order.
    pub fn records(&self) -> &[FSERecord] {
        &self.records
    }

    /// Returns the number of records in the batch.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true when the batch contains no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the record for the given row identifier.
    pub fn record_for_row_id(&self, row_id: RowId) -> Option<&FSERecord> {
        let index = self.row_index_for_row_id(row_id)?;

        self.records.get(index)
    }

    /// Returns the batch position for the given row identifier.
    pub fn row_index_for_row_id(&self, row_id: RowId) -> Option<usize> {
        self.row_index_by_id.get(&row_id).copied()
    }
}
