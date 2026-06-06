//! FSE-native typed predicates.

use std::error::Error;
use std::fmt;

use crate::data::{FSEFieldType, FSESchema, FSEValue};

/// Error returned when typed predicate validation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEPredicateError {
    /// A field index was outside the schema.
    FieldIndexOutOfRange {
        /// Referenced field index.
        field_index: usize,
        /// Number of fields in the schema.
        field_count: usize,
    },

    /// A field name was not present in the schema.
    UnknownFieldName {
        /// Referenced field name.
        name: String,
    },

    /// A predicate value was null.
    NullPredicateValue {
        /// Field index for the predicate.
        field: usize,
        /// Field name for the predicate.
        name: String,
    },

    /// A predicate value type did not match the schema field type.
    FieldTypeMismatch {
        /// Field index for the predicate.
        field: usize,
        /// Field name for the predicate.
        name: String,
        /// Field type required by the schema.
        expected: FSEFieldType,
        /// Field type found in the predicate value.
        actual: FSEFieldType,
    },

    /// A floating point predicate value was not finite.
    NonFiniteValue {
        /// Field index for the predicate.
        field: usize,
        /// Field name for the predicate.
        name: String,
    },

    /// A range predicate was used for a field type without ordered range semantics.
    UnsupportedRangeType {
        /// Field index for the predicate.
        field: usize,
        /// Field name for the predicate.
        name: String,
        /// Field type used by the range predicate.
        field_type: FSEFieldType,
    },

    /// A range predicate used different value types for minimum and maximum.
    RangeTypeMismatch {
        /// Field index for the predicate.
        field: usize,
        /// Field name for the predicate.
        name: String,
        /// Field type found in the minimum bound.
        min_type: FSEFieldType,
        /// Field type found in the maximum bound.
        max_type: FSEFieldType,
    },

    /// A range predicate minimum was greater than its maximum.
    InvertedRange {
        /// Field index for the predicate.
        field: usize,
        /// Field name for the predicate.
        name: String,
    },
}

impl fmt::Display for FSEPredicateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldIndexOutOfRange {
                field_index,
                field_count,
            } => write!(
                formatter,
                "field index {field_index} is outside schema field count {field_count}"
            ),
            Self::UnknownFieldName { name } => {
                write!(formatter, "schema field '{name}' was not found")
            }
            Self::NullPredicateValue { name, .. } => {
                write!(formatter, "predicate for field '{name}' must not be null")
            }
            Self::FieldTypeMismatch {
                name,
                expected,
                actual,
                ..
            } => write!(
                formatter,
                "predicate for field '{name}' expected {expected:?} but found {actual:?}"
            ),
            Self::NonFiniteValue { name, .. } => {
                write!(
                    formatter,
                    "predicate for field '{name}' must use finite values"
                )
            }
            Self::UnsupportedRangeType {
                name, field_type, ..
            } => write!(
                formatter,
                "range predicate for field '{name}' does not support {field_type:?}"
            ),
            Self::RangeTypeMismatch {
                name,
                min_type,
                max_type,
                ..
            } => write!(
                formatter,
                "range predicate for field '{name}' used {min_type:?} minimum and {max_type:?} maximum"
            ),
            Self::InvertedRange { name, .. } => {
                write!(
                    formatter,
                    "range predicate minimum must not exceed maximum for field '{name}'"
                )
            }
        }
    }
}

impl Error for FSEPredicateError {}

/// Field reference used by a typed predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEPredicateField {
    /// Field referenced by schema index.
    Index(usize),
    /// Field referenced by schema name.
    Name(String),
}

impl FSEPredicateField {
    /// Creates a field reference by index.
    pub fn index(index: usize) -> Self {
        Self::Index(index)
    }

    /// Creates a field reference by name.
    pub fn name(name: impl Into<String>) -> Self {
        Self::Name(name.into())
    }
}

/// Typed predicate operator before query-region compilation.
#[derive(Clone, Debug, PartialEq)]
pub enum FSEPredicateOperator {
    /// Field equality predicate.
    Equal(FSEValue),

    /// Closed range predicate.
    Range {
        /// Inclusive minimum predicate value.
        min: FSEValue,
        /// Inclusive maximum predicate value.
        max: FSEValue,
    },
}

/// Typed predicate before validation against a schema.
#[derive(Clone, Debug, PartialEq)]
pub struct FSEPredicate {
    field: FSEPredicateField,
    operator: FSEPredicateOperator,
}

impl FSEPredicate {
    /// Creates an equality predicate.
    pub fn equals(field: FSEPredicateField, value: FSEValue) -> Self {
        Self {
            field,
            operator: FSEPredicateOperator::Equal(value),
        }
    }

    /// Creates a closed range predicate.
    pub fn range(field: FSEPredicateField, min: FSEValue, max: FSEValue) -> Self {
        Self {
            field,
            operator: FSEPredicateOperator::Range { min, max },
        }
    }

    /// Returns the predicate field reference.
    pub fn field(&self) -> &FSEPredicateField {
        &self.field
    }

    /// Returns the predicate operator.
    pub fn operator(&self) -> &FSEPredicateOperator {
        &self.operator
    }

    /// Validates the predicate against a schema.
    pub fn validate(&self, schema: &FSESchema) -> Result<ValidatedFSEPredicate, FSEPredicateError> {
        let resolved = resolve_field(schema, &self.field)?;

        match &self.operator {
            FSEPredicateOperator::Equal(value) => {
                validate_value(resolved, value)?;

                Ok(ValidatedFSEPredicate {
                    field: resolved.index,
                    name: resolved.name.to_string(),
                    field_type: resolved.field_type,
                    operator: ValidatedFSEPredicateOperator::Equal(value.clone()),
                })
            }
            FSEPredicateOperator::Range { min, max } => {
                validate_range(resolved, min, max)?;

                Ok(ValidatedFSEPredicate {
                    field: resolved.index,
                    name: resolved.name.to_string(),
                    field_type: resolved.field_type,
                    operator: ValidatedFSEPredicateOperator::Range {
                        min: min.clone(),
                        max: max.clone(),
                    },
                })
            }
        }
    }
}

/// Typed predicate after validation against a schema.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedFSEPredicate {
    field: usize,
    name: String,
    field_type: FSEFieldType,
    operator: ValidatedFSEPredicateOperator,
}

impl ValidatedFSEPredicate {
    /// Returns the schema field index for the predicate.
    pub fn field(&self) -> usize {
        self.field
    }

    /// Returns the schema field name for the predicate.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the schema field type for the predicate.
    pub fn field_type(&self) -> FSEFieldType {
        self.field_type
    }

    /// Returns the validated predicate operator.
    pub fn operator(&self) -> &ValidatedFSEPredicateOperator {
        &self.operator
    }
}

/// Predicate operator after schema validation.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedFSEPredicateOperator {
    /// Validated equality predicate.
    Equal(FSEValue),

    /// Validated closed range predicate.
    Range {
        /// Inclusive minimum predicate value.
        min: FSEValue,
        /// Inclusive maximum predicate value.
        max: FSEValue,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedField<'a> {
    index: usize,
    name: &'a str,
    field_type: FSEFieldType,
}

fn resolve_field<'a>(
    schema: &'a FSESchema,
    field: &FSEPredicateField,
) -> Result<ResolvedField<'a>, FSEPredicateError> {
    match field {
        FSEPredicateField::Index(index) => {
            let Some(field) = schema.field(*index) else {
                return Err(FSEPredicateError::FieldIndexOutOfRange {
                    field_index: *index,
                    field_count: schema.len(),
                });
            };

            Ok(ResolvedField {
                index: *index,
                name: &field.name,
                field_type: field.field_type,
            })
        }
        FSEPredicateField::Name(name) => {
            let Some((index, field)) = schema
                .fields()
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == name.as_str())
            else {
                return Err(FSEPredicateError::UnknownFieldName { name: name.clone() });
            };

            Ok(ResolvedField {
                index,
                name: &field.name,
                field_type: field.field_type,
            })
        }
    }
}

fn validate_value(field: ResolvedField<'_>, value: &FSEValue) -> Result<(), FSEPredicateError> {
    let Some(actual) = value.field_type() else {
        return Err(FSEPredicateError::NullPredicateValue {
            field: field.index,
            name: field.name.to_string(),
        });
    };

    if actual != field.field_type {
        return Err(FSEPredicateError::FieldTypeMismatch {
            field: field.index,
            name: field.name.to_string(),
            expected: field.field_type,
            actual,
        });
    }

    if matches!(value, FSEValue::Float(value) if !value.is_finite()) {
        return Err(FSEPredicateError::NonFiniteValue {
            field: field.index,
            name: field.name.to_string(),
        });
    }

    Ok(())
}

fn validate_range(
    field: ResolvedField<'_>,
    min: &FSEValue,
    max: &FSEValue,
) -> Result<(), FSEPredicateError> {
    validate_value(field, min)?;
    validate_value(field, max)?;

    let min_type = min.field_type().expect("range minimum should be typed");
    let max_type = max.field_type().expect("range maximum should be typed");

    if min_type != max_type {
        return Err(FSEPredicateError::RangeTypeMismatch {
            field: field.index,
            name: field.name.to_string(),
            min_type,
            max_type,
        });
    }

    if !supports_range(field.field_type) {
        return Err(FSEPredicateError::UnsupportedRangeType {
            field: field.index,
            name: field.name.to_string(),
            field_type: field.field_type,
        });
    }

    if range_is_inverted(min, max) {
        return Err(FSEPredicateError::InvertedRange {
            field: field.index,
            name: field.name.to_string(),
        });
    }

    Ok(())
}

fn supports_range(field_type: FSEFieldType) -> bool {
    matches!(
        field_type,
        FSEFieldType::Integer | FSEFieldType::Float | FSEFieldType::TimestampMillis
    )
}

fn range_is_inverted(min: &FSEValue, max: &FSEValue) -> bool {
    match (min, max) {
        (FSEValue::Integer(min), FSEValue::Integer(max)) => min > max,
        (FSEValue::Float(min), FSEValue::Float(max)) => min > max,
        (FSEValue::TimestampMillis(min), FSEValue::TimestampMillis(max)) => min > max,
        _ => false,
    }
}
