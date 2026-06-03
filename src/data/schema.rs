//! FSE-native schema metadata.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use super::FSEFieldType;

/// Error returned when checked schema construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSESchemaError {
    /// No fields were provided.
    EmptyFields,

    /// A field name was empty.
    EmptyFieldName {
        /// Field index containing the empty name.
        index: usize,
    },

    /// A field name appeared more than once.
    DuplicateFieldName {
        /// Repeated field name.
        name: String,
    },
}

impl fmt::Display for FSESchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFields => formatter.write_str("schema must contain at least one field"),
            Self::EmptyFieldName { index } => {
                write!(formatter, "schema field {index} name must not be empty")
            }
            Self::DuplicateFieldName { name } => {
                write!(
                    formatter,
                    "schema field name '{name}' appears more than once"
                )
            }
        }
    }
}

impl Error for FSESchemaError {}

/// Field metadata for FSE-native records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FSEField {
    /// Stable field name.
    pub name: String,

    /// Logical field type.
    pub field_type: FSEFieldType,

    /// Whether the field accepts null values.
    pub nullable: bool,
}

impl FSEField {
    /// Creates field metadata.
    pub fn new(name: impl Into<String>, field_type: FSEFieldType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            field_type,
            nullable,
        }
    }
}

/// Schema metadata for FSE-native records.
///
/// # Runtime Role
///
/// `FSESchema` defines field order, field names, logical field types, and null
/// handling before semantic encoding maps records into coordinate space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FSESchema {
    fields: Vec<FSEField>,
}

impl FSESchema {
    /// Creates a schema from field metadata.
    ///
    /// # Panics
    ///
    /// Panics when the schema contains no fields, an empty field name, or a
    /// duplicate field name.
    pub fn new(fields: Vec<FSEField>) -> Self {
        Self::try_new(fields).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a schema and returns an error when field metadata is invalid.
    pub fn try_new(fields: Vec<FSEField>) -> Result<Self, FSESchemaError> {
        if fields.is_empty() {
            return Err(FSESchemaError::EmptyFields);
        }

        let mut names = HashSet::new();

        for (index, field) in fields.iter().enumerate() {
            if field.name.is_empty() {
                return Err(FSESchemaError::EmptyFieldName { index });
            }

            if !names.insert(field.name.as_str()) {
                return Err(FSESchemaError::DuplicateFieldName {
                    name: field.name.clone(),
                });
            }
        }

        Ok(Self { fields })
    }

    /// Returns field metadata in schema order.
    pub fn fields(&self) -> &[FSEField] {
        &self.fields
    }

    /// Returns the number of fields in the schema.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns true when the schema contains no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns the field at the given index.
    pub fn field(&self, index: usize) -> Option<&FSEField> {
        self.fields.get(index)
    }

    /// Returns the field with the given name.
    pub fn field_named(&self, name: &str) -> Option<&FSEField> {
        self.fields.iter().find(|field| field.name == name)
    }
}
