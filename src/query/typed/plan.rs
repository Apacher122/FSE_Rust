//! Typed query planning.

use std::error::Error;
use std::fmt;

use crate::data::{FSESchema, FSESchemaDimensionMapping, FSEValue};
use crate::encoding::{CategoricalDictionaryEncoder, FSERecordEncoderMetadataError};

use super::super::region::{QueryRegion, QueryRegionError};
use super::compiler::{
    FSEPredicateCompileError, compile_categorical_equality_predicate_to_query_region,
    compile_numeric_predicate_to_query_region, mapped_dimension,
};
use super::predicate::{
    FSEPredicate, FSEPredicateError, ValidatedFSEPredicate, ValidatedFSEPredicateOperator,
};

/// Error returned when typed query planning fails.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedQueryPlanError {
    /// Predicate validation failed.
    Predicate(FSEPredicateError),

    /// Predicate compilation failed.
    Compile(FSEPredicateCompileError),

    /// A categorical predicate had no encoder registered for its field.
    MissingCategoricalEncoder {
        /// Schema field index.
        field: usize,
        /// Schema field name.
        name: String,
    },

    /// Record encoder metadata could not be used for typed query planning.
    EncoderMetadata(FSERecordEncoderMetadataError),

    /// No plan components were provided for a conjunctive plan.
    EmptyConjunction,
}

impl fmt::Display for TypedQueryPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Predicate(error) => error.fmt(formatter),
            Self::Compile(error) => error.fmt(formatter),
            Self::MissingCategoricalEncoder { name, .. } => {
                write!(
                    formatter,
                    "categorical predicate for field '{name}' has no registered encoder"
                )
            }
            Self::EncoderMetadata(error) => error.fmt(formatter),
            Self::EmptyConjunction => {
                formatter.write_str("typed query plan conjunction requires at least one plan")
            }
        }
    }
}

impl Error for TypedQueryPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Predicate(error) => Some(error),
            Self::Compile(error) => Some(error),
            Self::EncoderMetadata(error) => Some(error),
            Self::MissingCategoricalEncoder { .. } => None,
            Self::EmptyConjunction => None,
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

impl From<FSERecordEncoderMetadataError> for TypedQueryPlanError {
    fn from(error: FSERecordEncoderMetadataError) -> Self {
        Self::EncoderMetadata(error)
    }
}

/// Typed query plan with geometric and exact predicate components.
///
/// # Runtime Role
///
/// `TypedQueryPlan` keeps the query region used for geometric pruning together
/// with the validated typed predicates used for exact logical evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedQueryPlan {
    predicates: Vec<ValidatedFSEPredicate>,
    query_region: QueryRegion,
    unsatisfiable: bool,
    categorical_equality_dimensions: Vec<TypedCategoricalEqualityDimension>,
}

impl TypedQueryPlan {
    /// Creates a typed query plan from already validated and compiled parts.
    pub fn new(predicate: ValidatedFSEPredicate, query_region: QueryRegion) -> Self {
        Self {
            predicates: vec![predicate],
            query_region,
            unsatisfiable: false,
            categorical_equality_dimensions: Vec::new(),
        }
    }

    /// Creates a typed query plan from existing plan components.
    ///
    /// # Runtime Role
    ///
    /// The resulting plan uses the geometric intersection of component query
    /// regions and stores every validated predicate for exact evaluation.
    pub fn conjunctive(plans: Vec<Self>) -> Result<Self, TypedQueryPlanError> {
        if plans.is_empty() {
            return Err(TypedQueryPlanError::EmptyConjunction);
        }

        let mut predicates = Vec::new();
        let mut categorical_equality_dimensions = Vec::new();
        let mut query_region = None;
        let mut unsatisfiable = false;

        for mut plan in plans {
            unsatisfiable |= plan.unsatisfiable;
            query_region = Some(match query_region {
                Some(existing) => {
                    let intersection = intersect_query_regions(&existing, &plan.query_region)?;
                    unsatisfiable |= intersection.unsatisfiable;
                    intersection.query_region
                }
                None => plan.query_region,
            });

            predicates.append(&mut plan.predicates);
            categorical_equality_dimensions.append(&mut plan.categorical_equality_dimensions);
        }

        Ok(Self {
            predicates,
            query_region: query_region.expect("non-empty plan list should produce query region"),
            unsatisfiable,
            categorical_equality_dimensions,
        })
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
        let field = predicate.field();
        let dimension = mapped_dimension(&predicate, mapping)?;
        let category = categorical_equality_category(&predicate).to_string();
        let query_region =
            compile_categorical_equality_predicate_to_query_region(&predicate, mapping, encoder)?;

        Ok(
            Self::new(predicate, query_region).with_categorical_equality_dimension(
                TypedCategoricalEqualityDimension::new(field, dimension, category),
            ),
        )
    }

    /// Returns the validated typed predicate.
    pub fn predicate(&self) -> &ValidatedFSEPredicate {
        &self.predicates[0]
    }

    /// Returns the validated typed predicates.
    pub fn predicates(&self) -> &[ValidatedFSEPredicate] {
        &self.predicates
    }

    /// Returns true when the typed predicate set has no satisfying rows.
    pub fn is_unsatisfiable(&self) -> bool {
        self.unsatisfiable
    }

    /// Returns the geometric query region used for pruning.
    pub fn query_region(&self) -> &QueryRegion {
        &self.query_region
    }

    pub(super) fn with_categorical_equality_dimension(
        mut self,
        categorical_equality_dimension: TypedCategoricalEqualityDimension,
    ) -> Self {
        self.categorical_equality_dimensions
            .push(categorical_equality_dimension);
        self
    }

    pub(super) fn categorical_equality_dimensions(&self) -> &[TypedCategoricalEqualityDimension] {
        &self.categorical_equality_dimensions
    }
}

/// Categorical equality predicate metadata used by planning estimates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TypedCategoricalEqualityDimension {
    field: usize,
    dimension: usize,
    category: String,
}

impl TypedCategoricalEqualityDimension {
    pub(super) fn new(field: usize, dimension: usize, category: String) -> Self {
        Self {
            field,
            dimension,
            category,
        }
    }

    pub(super) fn field(&self) -> usize {
        self.field
    }

    pub(super) fn dimension(&self) -> usize {
        self.dimension
    }

    pub(super) fn category(&self) -> &str {
        &self.category
    }
}

#[derive(Clone, Debug, PartialEq)]
struct QueryRegionIntersection {
    query_region: QueryRegion,
    unsatisfiable: bool,
}

fn intersect_query_regions(
    left: &QueryRegion,
    right: &QueryRegion,
) -> Result<QueryRegionIntersection, TypedQueryPlanError> {
    if left.dimensions() != right.dimensions() {
        return Err(
            FSEPredicateCompileError::QueryRegion(QueryRegionError::DimensionMismatch {
                min_dimensions: left.dimensions(),
                max_dimensions: right.dimensions(),
            })
            .into(),
        );
    }

    let mut min = Vec::with_capacity(left.dimensions());
    let mut max = Vec::with_capacity(left.dimensions());
    let mut unsatisfiable = false;

    for dimension in 0..left.dimensions() {
        let left_min = left.min[dimension];
        let left_max = left.max[dimension];
        let right_min = right.min[dimension];
        let right_max = right.max[dimension];
        let lower = left_min.max(right_min);
        let upper = left_max.min(right_max);

        if lower > upper {
            unsatisfiable = true;
            min.push(lower);
            max.push(lower);
        } else {
            min.push(lower);
            max.push(upper);
        }
    }

    Ok(QueryRegionIntersection {
        query_region: QueryRegion::try_new(min, max).map_err(FSEPredicateCompileError::from)?,
        unsatisfiable,
    })
}

pub(super) fn categorical_equality_category(predicate: &ValidatedFSEPredicate) -> &str {
    match predicate.operator() {
        ValidatedFSEPredicateOperator::Equal(FSEValue::Category(category)) => category,
        _ => unreachable!("validated categorical equality predicate should contain a category"),
    }
}
