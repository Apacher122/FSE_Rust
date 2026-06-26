//! Typed predicate compilation.

use std::error::Error;
use std::fmt;

use crate::data::{FSEFieldType, FSESchemaDimensionMapping, FSEValue};
use crate::encoding::CategoricalDictionaryEncoder;
use crate::math::Scalar;

use super::super::region::{QueryRegion, QueryRegionError};
use super::predicate::{ValidatedFSEPredicate, ValidatedFSEPredicateOperator};

/// Error returned when a typed predicate cannot be compiled into a query region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEPredicateCompileError {
    /// The predicate field has no coordinate mapping.
    FieldNotMapped {
        /// Schema field index.
        field: usize,
        /// Schema field name.
        name: String,
    },

    /// The predicate field maps to more than one coordinate dimension.
    FieldMappedToMultipleDimensions {
        /// Schema field index.
        field: usize,
        /// Schema field name.
        name: String,
        /// Number of coordinate dimensions mapped to the field.
        dimensions: usize,
    },

    /// The predicate field type is not supported by the numeric compiler.
    UnsupportedFieldType {
        /// Schema field index.
        field: usize,
        /// Schema field name.
        name: String,
        /// Unsupported field type.
        field_type: FSEFieldType,
    },

    /// The predicate field type is not supported by the categorical compiler.
    UnsupportedCategoricalFieldType {
        /// Schema field index.
        field: usize,
        /// Schema field name.
        name: String,
        /// Unsupported field type.
        field_type: FSEFieldType,
    },

    /// A categorical predicate used an operator other than equality.
    UnsupportedCategoricalOperator {
        /// Schema field index.
        field: usize,
        /// Schema field name.
        name: String,
    },

    /// A categorical equality predicate referenced a category outside the dictionary.
    UnknownCategory {
        /// Schema field index.
        field: usize,
        /// Schema field name.
        name: String,
        /// Category label referenced by the predicate.
        category: String,
    },

    /// Query-region construction failed after compilation.
    QueryRegion(QueryRegionError),
}

impl fmt::Display for FSEPredicateCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldNotMapped { name, .. } => {
                write!(
                    formatter,
                    "predicate field '{name}' has no coordinate mapping"
                )
            }
            Self::FieldMappedToMultipleDimensions {
                name, dimensions, ..
            } => write!(
                formatter,
                "predicate field '{name}' maps to {dimensions} coordinate dimensions"
            ),
            Self::UnsupportedFieldType {
                name, field_type, ..
            } => write!(
                formatter,
                "predicate field '{name}' with type {field_type:?} cannot be compiled by the numeric predicate compiler"
            ),
            Self::UnsupportedCategoricalFieldType {
                name, field_type, ..
            } => write!(
                formatter,
                "predicate field '{name}' with type {field_type:?} cannot be compiled by the categorical predicate compiler"
            ),
            Self::UnsupportedCategoricalOperator { name, .. } => {
                write!(
                    formatter,
                    "categorical predicate for field '{name}' must use equality"
                )
            }
            Self::UnknownCategory { name, category, .. } => write!(
                formatter,
                "category '{category}' for field '{name}' is not in dictionary"
            ),
            Self::QueryRegion(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSEPredicateCompileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::QueryRegion(error) => Some(error),
            _ => None,
        }
    }
}

impl From<QueryRegionError> for FSEPredicateCompileError {
    fn from(error: QueryRegionError) -> Self {
        Self::QueryRegion(error)
    }
}

/// Compiles a validated numeric predicate into a query region.
///
/// # Runtime Role
///
/// This function maps a typed equality or range predicate to the coordinate
/// dimension identified by schema mapping metadata. Dimensions not constrained
/// by the predicate are assigned finite open-span bounds.
pub fn compile_numeric_predicate_to_query_region(
    predicate: &ValidatedFSEPredicate,
    mapping: &FSESchemaDimensionMapping,
) -> Result<QueryRegion, FSEPredicateCompileError> {
    ensure_numeric_field(predicate)?;

    let dimension = mapped_dimension(predicate, mapping)?;
    let dimensions = coordinate_dimensions(mapping);
    let mut min = vec![Scalar::MIN; dimensions];
    let mut max = vec![Scalar::MAX; dimensions];

    match predicate.operator() {
        ValidatedFSEPredicateOperator::Equal(value) => {
            let encoded = numeric_value_to_scalar(value);
            min[dimension] = encoded;
            max[dimension] = encoded;
        }
        ValidatedFSEPredicateOperator::Range {
            min: lower,
            max: upper,
        } => {
            min[dimension] = numeric_value_to_scalar(lower);
            max[dimension] = numeric_value_to_scalar(upper);
        }
    }

    Ok(QueryRegion::try_new(min, max)?)
}

/// Compiles a validated categorical equality predicate into a query region.
///
/// # Runtime Role
///
/// This function maps a typed categorical equality predicate through the same
/// dictionary encoder used for record encoding, then constrains the mapped
/// coordinate dimension to the encoded category code.
pub fn compile_categorical_equality_predicate_to_query_region(
    predicate: &ValidatedFSEPredicate,
    mapping: &FSESchemaDimensionMapping,
    encoder: &CategoricalDictionaryEncoder,
) -> Result<QueryRegion, FSEPredicateCompileError> {
    ensure_categorical_field(predicate)?;

    let dimension = mapped_dimension(predicate, mapping)?;
    let dimensions = coordinate_dimensions(mapping);
    let mut min = vec![Scalar::MIN; dimensions];
    let mut max = vec![Scalar::MAX; dimensions];

    let category = match predicate.operator() {
        ValidatedFSEPredicateOperator::Equal(FSEValue::Category(category)) => category,
        ValidatedFSEPredicateOperator::Equal(_) => unreachable!(
            "validated categorical predicate should contain categorical equality value"
        ),
        ValidatedFSEPredicateOperator::Range { .. } => {
            return Err(FSEPredicateCompileError::UnsupportedCategoricalOperator {
                field: predicate.field(),
                name: predicate.name().to_string(),
            });
        }
    };

    let Some(code) = encoder.code_for_category(category) else {
        return Err(FSEPredicateCompileError::UnknownCategory {
            field: predicate.field(),
            name: predicate.name().to_string(),
            category: category.clone(),
        });
    };

    let encoded = code as Scalar;
    min[dimension] = encoded;
    max[dimension] = encoded;

    Ok(QueryRegion::try_new(min, max)?)
}

fn ensure_numeric_field(predicate: &ValidatedFSEPredicate) -> Result<(), FSEPredicateCompileError> {
    if matches!(
        predicate.field_type(),
        FSEFieldType::Integer | FSEFieldType::Float | FSEFieldType::TimestampMillis
    ) {
        return Ok(());
    }

    Err(FSEPredicateCompileError::UnsupportedFieldType {
        field: predicate.field(),
        name: predicate.name().to_string(),
        field_type: predicate.field_type(),
    })
}

fn ensure_categorical_field(
    predicate: &ValidatedFSEPredicate,
) -> Result<(), FSEPredicateCompileError> {
    if predicate.field_type() == FSEFieldType::Category {
        return Ok(());
    }

    Err(FSEPredicateCompileError::UnsupportedCategoricalFieldType {
        field: predicate.field(),
        name: predicate.name().to_string(),
        field_type: predicate.field_type(),
    })
}

pub(super) fn mapped_dimension(
    predicate: &ValidatedFSEPredicate,
    mapping: &FSESchemaDimensionMapping,
) -> Result<usize, FSEPredicateCompileError> {
    let dimensions = mapping.mappings_for_field(predicate.field());

    match dimensions.as_slice() {
        [] => Err(FSEPredicateCompileError::FieldNotMapped {
            field: predicate.field(),
            name: predicate.name().to_string(),
        }),
        [dimension] => Ok(dimension.dimension),
        _ => Err(FSEPredicateCompileError::FieldMappedToMultipleDimensions {
            field: predicate.field(),
            name: predicate.name().to_string(),
            dimensions: dimensions.len(),
        }),
    }
}

fn coordinate_dimensions(mapping: &FSESchemaDimensionMapping) -> usize {
    mapping
        .mappings()
        .iter()
        .map(|mapping| mapping.dimension)
        .max()
        .map_or(0, |dimension| dimension + 1)
}

fn numeric_value_to_scalar(value: &FSEValue) -> Scalar {
    match value {
        FSEValue::Integer(value) => *value as Scalar,
        FSEValue::Float(value) => *value as Scalar,
        FSEValue::TimestampMillis(value) => *value as Scalar,
        _ => unreachable!("validated numeric predicate should contain numeric values"),
    }
}
