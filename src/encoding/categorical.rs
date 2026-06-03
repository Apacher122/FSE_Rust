//! Categorical dictionary encoders.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::data::{FSEFieldType, FSEValue};

use super::{EncodedCoordinates, FSEEncodingError, FSEFieldEncoder};

/// Error returned when checked categorical dictionary construction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoricalDictionaryError {
    /// No category labels were provided.
    EmptyDictionary,

    /// A category label was empty.
    EmptyCategory {
        /// Category index containing the empty label.
        index: usize,
    },

    /// A category label appeared more than once.
    DuplicateCategory {
        /// Repeated category label.
        category: String,
    },
}

impl fmt::Display for CategoricalDictionaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDictionary => {
                formatter.write_str("categorical dictionary must contain at least one category")
            }
            Self::EmptyCategory { index } => {
                write!(formatter, "category {index} must not be empty")
            }
            Self::DuplicateCategory { category } => {
                write!(formatter, "category '{category}' appears more than once")
            }
        }
    }
}

impl Error for CategoricalDictionaryError {}

/// Dictionary-backed encoder for categorical values.
///
/// # Runtime Role
///
/// `CategoricalDictionaryEncoder` maps stable category labels to deterministic
/// numeric codes. The mapping is exact for equality predicates; it does not
/// imply lexical ordering or semantic similarity.
#[derive(Clone, Debug, PartialEq)]
pub struct CategoricalDictionaryEncoder {
    categories: Vec<String>,
    codes_by_category: HashMap<String, usize>,
}

impl CategoricalDictionaryEncoder {
    /// Creates a categorical dictionary encoder.
    ///
    /// # Panics
    ///
    /// Panics when the dictionary is empty, contains an empty category, or
    /// contains duplicate categories.
    pub fn new(categories: Vec<String>) -> Self {
        Self::try_new(categories).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a categorical dictionary encoder and returns an error when invalid.
    pub fn try_new(categories: Vec<String>) -> Result<Self, CategoricalDictionaryError> {
        if categories.is_empty() {
            return Err(CategoricalDictionaryError::EmptyDictionary);
        }

        let mut codes_by_category = HashMap::new();

        for (index, category) in categories.iter().enumerate() {
            if category.is_empty() {
                return Err(CategoricalDictionaryError::EmptyCategory { index });
            }

            if codes_by_category.insert(category.clone(), index).is_some() {
                return Err(CategoricalDictionaryError::DuplicateCategory {
                    category: category.clone(),
                });
            }
        }

        Ok(Self {
            categories,
            codes_by_category,
        })
    }

    /// Returns category labels in dictionary order.
    pub fn categories(&self) -> &[String] {
        &self.categories
    }

    /// Returns the numeric code for a category label.
    pub fn code_for_category(&self, category: &str) -> Option<usize> {
        self.codes_by_category.get(category).copied()
    }
}

impl FSEFieldEncoder for CategoricalDictionaryEncoder {
    fn field_type(&self) -> FSEFieldType {
        FSEFieldType::Category
    }

    fn output_dimensions(&self) -> usize {
        1
    }

    fn encode_value(&self, value: &FSEValue) -> Result<EncodedCoordinates, FSEEncodingError> {
        match value {
            FSEValue::Category(category) => {
                let Some(code) = self.code_for_category(category) else {
                    return Err(FSEEncodingError::UnsupportedValue {
                        reason: format!("category '{category}' is not in dictionary"),
                    });
                };

                Ok(EncodedCoordinates::new(vec![code as f32]))
            }
            FSEValue::Null => Err(FSEEncodingError::NullValue),
            other => Err(FSEEncodingError::FieldTypeMismatch {
                expected: FSEFieldType::Category,
                actual: other
                    .field_type()
                    .expect("non-null value should have field type"),
            }),
        }
    }
}
