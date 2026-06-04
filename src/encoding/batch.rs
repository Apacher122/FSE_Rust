//! Batch encoding for typed records.

use std::error::Error;
use std::fmt;

use crate::data::{FSERecordBatch, RowId};
use crate::math::{Vector, VectorError};

use super::{FSEEncodingError, FSERecordEncoder};

/// Error returned when checked record batch encoding fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERecordBatchEncodingError {
    /// A record could not be encoded.
    RecordEncoding {
        /// Record index in batch order.
        record: usize,
        /// Stable row identifier for the record.
        row_id: RowId,
        /// Encoding error returned for the record.
        source: FSEEncodingError,
    },

    /// Encoded coordinates could not be represented as a geometry vector.
    InvalidVector {
        /// Record index in batch order.
        record: usize,
        /// Stable row identifier for the record.
        row_id: RowId,
        /// Vector construction error.
        source: VectorError,
    },

    /// A record produced a coordinate count different from the encoder contract.
    DimensionMismatch {
        /// Record index in batch order.
        record: usize,
        /// Stable row identifier for the record.
        row_id: RowId,
        /// Number of coordinates required by the encoder.
        expected: usize,
        /// Number of coordinates produced by the record.
        actual: usize,
    },
}

impl fmt::Display for FSERecordBatchEncodingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordEncoding { record, row_id, .. } => write!(
                formatter,
                "record {record} with row id {} could not be encoded",
                row_id.value()
            ),
            Self::InvalidVector { record, row_id, .. } => write!(
                formatter,
                "record {record} with row id {} produced invalid coordinates",
                row_id.value()
            ),
            Self::DimensionMismatch {
                record,
                row_id,
                expected,
                actual,
            } => write!(
                formatter,
                "record {record} with row id {} produced {actual} coordinates but encoder requires {expected}",
                row_id.value()
            ),
        }
    }
}

impl Error for FSERecordBatchEncodingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RecordEncoding { source, .. } => Some(source),
            Self::InvalidVector { source, .. } => Some(source),
            Self::DimensionMismatch { .. } => None,
        }
    }
}

/// Encoded batch of row identifiers and coordinate vectors.
///
/// # Runtime Role
///
/// `EncodedRecordBatch` preserves stable row identity beside coordinate vectors
/// before the vectors are passed into the geometry and build layers.
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedRecordBatch {
    row_ids: Vec<RowId>,
    vectors: Vec<Vector>,
    dimensions: usize,
}

impl EncodedRecordBatch {
    /// Creates an encoded record batch from row identifiers and vectors.
    ///
    /// # Panics
    ///
    /// Panics when the row identifier count does not match the vector count or
    /// when vector dimensions are inconsistent.
    pub fn new(row_ids: Vec<RowId>, vectors: Vec<Vector>) -> Self {
        Self::try_new(row_ids, vectors).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates an encoded record batch and returns an error when invalid.
    pub fn try_new(
        row_ids: Vec<RowId>,
        vectors: Vec<Vector>,
    ) -> Result<Self, FSERecordBatchEncodingError> {
        let dimensions = vectors.first().map_or(0, Vector::dimensions);

        if row_ids.len() != vectors.len() {
            let row_id = row_ids.first().copied().unwrap_or_else(|| RowId::new(0));
            return Err(FSERecordBatchEncodingError::DimensionMismatch {
                record: vectors.len(),
                row_id,
                expected: row_ids.len(),
                actual: vectors.len(),
            });
        }

        for (record, (row_id, vector)) in row_ids.iter().zip(vectors.iter()).enumerate() {
            if vector.dimensions() != dimensions {
                return Err(FSERecordBatchEncodingError::DimensionMismatch {
                    record,
                    row_id: *row_id,
                    expected: dimensions,
                    actual: vector.dimensions(),
                });
            }
        }

        Ok(Self {
            row_ids,
            vectors,
            dimensions,
        })
    }

    /// Returns row identifiers in vector order.
    pub fn row_ids(&self) -> &[RowId] {
        &self.row_ids
    }

    /// Returns encoded coordinate vectors in row identifier order.
    pub fn vectors(&self) -> &[Vector] {
        &self.vectors
    }

    /// Returns the number of vectors in the batch.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Returns true when the batch contains no vectors.
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Returns the coordinate dimension count for each vector.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

/// Encodes a typed record batch into coordinate vectors.
///
/// # Runtime Role
///
/// This function is the checked boundary between FSE-native typed records and
/// the numeric geometry layer.
pub fn encode_record_batch(
    batch: &FSERecordBatch,
    encoder: &impl FSERecordEncoder,
) -> Result<EncodedRecordBatch, FSERecordBatchEncodingError> {
    let expected_dimensions = encoder.output_dimensions();
    let mut vectors = Vec::with_capacity(batch.len());

    for (record, (row_id, typed_record)) in batch
        .row_ids()
        .iter()
        .zip(batch.records().iter())
        .enumerate()
    {
        let coordinates = encoder.encode_record(typed_record).map_err(|source| {
            FSERecordBatchEncodingError::RecordEncoding {
                record,
                row_id: *row_id,
                source,
            }
        })?;

        let actual_dimensions = coordinates.dimensions();

        if actual_dimensions != expected_dimensions {
            return Err(FSERecordBatchEncodingError::DimensionMismatch {
                record,
                row_id: *row_id,
                expected: expected_dimensions,
                actual: actual_dimensions,
            });
        }

        let vector = Vector::try_new(coordinates.values().to_vec()).map_err(|source| {
            FSERecordBatchEncodingError::InvalidVector {
                record,
                row_id: *row_id,
                source,
            }
        })?;

        vectors.push(vector);
    }

    Ok(EncodedRecordBatch {
        row_ids: batch.row_ids().to_vec(),
        vectors,
        dimensions: expected_dimensions,
    })
}
