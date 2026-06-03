//! Schema-to-coordinate dimensional mapping metadata.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use super::FSESchema;

/// Error returned when checked dimensional mapping construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSESchemaDimensionMappingError {
    /// No mappings were provided.
    EmptyMappings,

    /// A mapping referenced a field index outside the schema.
    FieldIndexOutOfRange {
        /// Referenced field index.
        field_index: usize,
        /// Number of fields in the schema.
        field_count: usize,
    },

    /// More than one mapping used the same coordinate dimension.
    DuplicateDimension {
        /// Duplicate coordinate dimension.
        dimension: usize,
    },
}

impl fmt::Display for FSESchemaDimensionMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMappings => {
                formatter.write_str("schema dimensional mapping must contain at least one mapping")
            }
            Self::FieldIndexOutOfRange {
                field_index,
                field_count,
            } => write!(
                formatter,
                "field index {field_index} is outside schema field count {field_count}"
            ),
            Self::DuplicateDimension { dimension } => {
                write!(
                    formatter,
                    "coordinate dimension {dimension} appears more than once"
                )
            }
        }
    }
}

impl Error for FSESchemaDimensionMappingError {}

/// Mapping from one schema field to one coordinate dimension.
///
/// # Runtime Role
///
/// This metadata identifies which logical field contributes to a coordinate
/// dimension after semantic encoding. A field may appear in more than one
/// mapping when an encoder expands one logical field into multiple dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSEDimensionMapping {
    /// Schema field index.
    pub field_index: usize,

    /// Coordinate dimension index.
    pub dimension: usize,
}

impl FSEDimensionMapping {
    /// Creates dimensional mapping metadata.
    pub fn new(field_index: usize, dimension: usize) -> Self {
        Self {
            field_index,
            dimension,
        }
    }
}

/// Validated dimensional mapping for a schema.
///
/// # Runtime Role
///
/// `FSESchemaDimensionMapping` describes how typed schema fields correspond to
/// numeric coordinate dimensions before semantic encoding is implemented.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FSESchemaDimensionMapping {
    mappings: Vec<FSEDimensionMapping>,
}

impl FSESchemaDimensionMapping {
    /// Creates dimensional mapping metadata.
    ///
    /// # Panics
    ///
    /// Panics when no mappings are provided, a field index is outside the
    /// schema, or a coordinate dimension appears more than once.
    pub fn new(schema: &FSESchema, mappings: Vec<FSEDimensionMapping>) -> Self {
        Self::try_new(schema, mappings).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates dimensional mapping metadata and returns an error when invalid.
    pub fn try_new(
        schema: &FSESchema,
        mappings: Vec<FSEDimensionMapping>,
    ) -> Result<Self, FSESchemaDimensionMappingError> {
        if mappings.is_empty() {
            return Err(FSESchemaDimensionMappingError::EmptyMappings);
        }

        let mut dimensions = HashSet::new();

        for mapping in &mappings {
            if mapping.field_index >= schema.len() {
                return Err(FSESchemaDimensionMappingError::FieldIndexOutOfRange {
                    field_index: mapping.field_index,
                    field_count: schema.len(),
                });
            }

            if !dimensions.insert(mapping.dimension) {
                return Err(FSESchemaDimensionMappingError::DuplicateDimension {
                    dimension: mapping.dimension,
                });
            }
        }

        Ok(Self { mappings })
    }

    /// Returns mapping entries in declared order.
    pub fn mappings(&self) -> &[FSEDimensionMapping] {
        &self.mappings
    }

    /// Returns the number of mapped coordinate dimensions.
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Returns true when there are no dimensional mappings.
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Returns the mapping for a coordinate dimension.
    pub fn mapping_for_dimension(&self, dimension: usize) -> Option<&FSEDimensionMapping> {
        self.mappings
            .iter()
            .find(|mapping| mapping.dimension == dimension)
    }

    /// Returns all mappings for a schema field.
    pub fn mappings_for_field(&self, field_index: usize) -> Vec<&FSEDimensionMapping> {
        self.mappings
            .iter()
            .filter(|mapping| mapping.field_index == field_index)
            .collect()
    }
}
