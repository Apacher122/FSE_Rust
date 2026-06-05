//! Typed query planning.

use std::error::Error;
use std::fmt;

use crate::data::{FSESchema, FSESchemaDimensionMapping};
use crate::encoding::CategoricalDictionaryEncoder;

use super::{
    FSEPredicate, FSEPredicateCompileError, FSEPredicateError, QueryRegion, ValidatedFSEPredicate,
    compile_categorical_equality_predicate_to_query_region,
    compile_numeric_predicate_to_query_region,
};

/// Error returned when typed query planning fails.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedQueryPlanError {
    /// Predicate validation failed.
    Predicate(FSEPredicateError),

    /// Predicate compilation failed.
    Compile(FSEPredicateCompileError),
}

impl fmt::Display for TypedQueryPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Predicate(error) => error.fmt(formatter),
            Self::Compile(error) => error.fmt(formatter),
        }
    }
}

impl Error for TypedQueryPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Predicate(error) => Some(error),
            Self::Compile(error) => Some(error),
        }
    }
}

impl From<FSEPredicateError> for TypedQueryPlanError {
    fn from(error: FSEPredicateError) -> Self {
        Self::Predicate(error)
    }
}

impl From<FSEPredicateCompileError> for TypedQueryPlanError {
    fn from(error: FSEPredicateCompileError) -> Self {
        Self::Compile(error)
    }
}

/// Typed query plan with geometric and exact predicate components.
///
/// # Runtime Role
///
/// `TypedQueryPlan` keeps the query region used for geometric pruning together
/// with the validated typed predicate used for exact logical evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedQueryPlan {
    predicate: ValidatedFSEPredicate,
    query_region: QueryRegion,
}

impl TypedQueryPlan {
    /// Creates a typed query plan from already validated and compiled parts.
    pub fn new(predicate: ValidatedFSEPredicate, query_region: QueryRegion) -> Self {
        Self {
            predicate,
            query_region,
        }
    }

    /// Creates a typed query plan for a numeric predicate.
    pub fn numeric(
        predicate: &FSEPredicate,
        schema: &FSESchema,
        mapping: &FSESchemaDimensionMapping,
    ) -> Result<Self, TypedQueryPlanError> {
        let predicate = predicate.validate(schema)?;
        let query_region = compile_numeric_predicate_to_query_region(&predicate, mapping)?;

        Ok(Self::new(predicate, query_region))
    }

    /// Creates a typed query plan for a categorical equality predicate.
    pub fn categorical_equality(
        predicate: &FSEPredicate,
        schema: &FSESchema,
        mapping: &FSESchemaDimensionMapping,
        encoder: &CategoricalDictionaryEncoder,
    ) -> Result<Self, TypedQueryPlanError> {
        let predicate = predicate.validate(schema)?;
        let query_region =
            compile_categorical_equality_predicate_to_query_region(&predicate, mapping, encoder)?;

        Ok(Self::new(predicate, query_region))
    }

    /// Returns the validated typed predicate.
    pub fn predicate(&self) -> &ValidatedFSEPredicate {
        &self.predicate
    }

    /// Returns the geometric query region used for pruning.
    pub fn query_region(&self) -> &QueryRegion {
        &self.query_region
    }
}
