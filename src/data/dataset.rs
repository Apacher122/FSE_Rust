//! Typed dataset metadata.

use std::error::Error;
use std::fmt;

use super::{FSESchema, FSESchemaDimensionMapping, FSESchemaDimensionMappingError};

/// Error returned when checked typed dataset metadata construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEDatasetMetadataError {
    /// The dataset name was empty.
    EmptyName,

    /// The dimensional mapping was invalid for the schema.
    DimensionMapping(FSESchemaDimensionMappingError),
}

impl fmt::Display for FSEDatasetMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("dataset name must not be empty"),
            Self::DimensionMapping(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSEDatasetMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EmptyName => None,
            Self::DimensionMapping(error) => Some(error),
        }
    }
}

impl From<FSESchemaDimensionMappingError> for FSEDatasetMetadataError {
    fn from(error: FSESchemaDimensionMappingError) -> Self {
        Self::DimensionMapping(error)
    }
}

/// Metadata for a typed FSE dataset.
///
/// # Runtime Role
///
/// `FSEDatasetMetadata` ties a dataset name, schema, dimensional mapping, and
/// record count together before semantic encoding creates coordinate records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FSEDatasetMetadata {
    name: String,
    schema: FSESchema,
    dimension_mapping: FSESchemaDimensionMapping,
    record_count: usize,
}

impl FSEDatasetMetadata {
    /// Creates typed dataset metadata.
    ///
    /// # Panics
    ///
    /// Panics when the dataset name is empty or when the dimensional mapping is
    /// invalid for the schema.
    pub fn new(
        name: impl Into<String>,
        schema: FSESchema,
        mappings: Vec<super::FSEDimensionMapping>,
        record_count: usize,
    ) -> Self {
        Self::try_new(name, schema, mappings, record_count)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates typed dataset metadata and returns an error when metadata is invalid.
    pub fn try_new(
        name: impl Into<String>,
        schema: FSESchema,
        mappings: Vec<super::FSEDimensionMapping>,
        record_count: usize,
    ) -> Result<Self, FSEDatasetMetadataError> {
        let name = name.into();

        if name.is_empty() {
            return Err(FSEDatasetMetadataError::EmptyName);
        }

        let dimension_mapping = FSESchemaDimensionMapping::try_new(&schema, mappings)?;

        Ok(Self {
            name,
            schema,
            dimension_mapping,
            record_count,
        })
    }

    /// Returns the dataset name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the dataset schema.
    pub fn schema(&self) -> &FSESchema {
        &self.schema
    }

    /// Returns the schema-to-coordinate dimensional mapping.
    pub fn dimension_mapping(&self) -> &FSESchemaDimensionMapping {
        &self.dimension_mapping
    }

    /// Returns the number of logical records described by the metadata.
    pub fn record_count(&self) -> usize {
        self.record_count
    }
}
