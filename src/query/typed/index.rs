//! Typed indexed query interface.

use std::error::Error;
use std::fmt;

use crate::build::{BuildInputError, FSEBuilder, RowMappedFSEIndex};
use crate::data::{FSERecordBatch, RowId};
use crate::encoding::{FSERecordBatchEncodingError, FSERecordEncoder, encode_record_batch};

use super::execution::{
    IndexedTypedQueryError, IndexedTypedQueryReport, IndexedTypedQueryRowReport,
    TypedQueryResultRow, evaluate_indexed_typed_query_plan, evaluate_indexed_typed_query_plan_rows,
    evaluate_indexed_typed_query_plan_rows_with_stats,
    evaluate_indexed_typed_query_plan_with_stats,
};
use super::plan::TypedQueryPlan;

/// Error returned when typed indexed query construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedQueryIndexBuildError {
    /// Record batch encoding failed.
    Encoding(FSERecordBatchEncodingError),

    /// Index construction rejected the encoded batch.
    Build(BuildInputError),
}

impl fmt::Display for TypedQueryIndexBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding(error) => error.fmt(formatter),
            Self::Build(error) => error.fmt(formatter),
        }
    }
}

impl Error for TypedQueryIndexBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encoding(error) => Some(error),
            Self::Build(error) => Some(error),
        }
    }
}

impl From<FSERecordBatchEncodingError> for TypedQueryIndexBuildError {
    fn from(error: FSERecordBatchEncodingError) -> Self {
        Self::Encoding(error)
    }
}

impl From<BuildInputError> for TypedQueryIndexBuildError {
    fn from(error: BuildInputError) -> Self {
        Self::Build(error)
    }
}

/// Typed records paired with a row-mapped FSE index.
///
/// # Runtime Role
///
/// `TypedQueryIndex` keeps typed records beside the geometric index that was
/// built from their encoded coordinates. Query execution uses the index for
/// pruning and the record batch for exact typed predicate evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedQueryIndex {
    batch: FSERecordBatch,
    index: RowMappedFSEIndex,
}

impl TypedQueryIndex {
    /// Creates a typed query index from existing parts.
    pub fn from_parts(batch: FSERecordBatch, index: RowMappedFSEIndex) -> Self {
        Self { batch, index }
    }

    /// Encodes a record batch and builds a row-mapped index for typed queries.
    pub fn try_build(
        batch: FSERecordBatch,
        encoder: &impl FSERecordEncoder,
        builder: &FSEBuilder,
    ) -> Result<Self, TypedQueryIndexBuildError> {
        let encoded = encode_record_batch(&batch, encoder)?;
        let index = builder.try_build_row_mapped_encoded_batch(&encoded)?;

        Ok(Self { batch, index })
    }

    /// Returns the typed record batch.
    pub fn batch(&self) -> &FSERecordBatch {
        &self.batch
    }

    /// Returns the row-mapped FSE index.
    pub fn index(&self) -> &RowMappedFSEIndex {
        &self.index
    }

    /// Evaluates a typed query plan and returns matching row identifiers.
    pub fn query_row_ids(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan(&self.index, &self.batch, plan)
    }

    /// Evaluates a typed query plan and returns row identifiers with statistics.
    pub fn query_row_ids_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_with_stats(&self.index, &self.batch, plan)
    }

    /// Evaluates a typed query plan and returns matching typed rows.
    pub fn query_rows(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_rows(&self.index, &self.batch, plan)
    }

    /// Evaluates a typed query plan and returns typed rows with statistics.
    pub fn query_rows_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
        evaluate_indexed_typed_query_plan_rows_with_stats(&self.index, &self.batch, plan)
    }
}
