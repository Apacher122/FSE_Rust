//! Typed query plan builder.

use std::collections::HashMap;

use crate::data::{FSEFieldType, FSESchema, FSESchemaDimensionMapping};
use crate::encoding::{
    CategoricalDictionaryEncoder, FSEFieldEncoderMetadata, FSERecordEncoderMetadata,
};

use super::compiler::{
    FSEPredicateCompileError, compile_categorical_equality_predicate_to_query_region,
    compile_numeric_predicate_to_query_region,
};
use super::plan::{TypedQueryPlan, TypedQueryPlanError};
use super::predicate::FSEPredicate;

/// Builder for typed query plans.
///
/// # Runtime Role
///
/// `TypedQueryPlanBuilder` validates typed predicates against schema metadata,
/// compiles them into query regions using the schema-to-dimension mapping, and
/// returns a conjunctive typed query plan.
#[derive(Clone, Debug)]
pub struct TypedQueryPlanBuilder<'a> {
    schema: &'a FSESchema,
    mapping: &'a FSESchemaDimensionMapping,
    categorical_encoders: HashMap<usize, CategoricalDictionaryEncoder>,
    predicates: Vec<FSEPredicate>,
}

impl<'a> TypedQueryPlanBuilder<'a> {
    /// Creates a builder for the provided schema and dimensional mapping.
    pub fn new(schema: &'a FSESchema, mapping: &'a FSESchemaDimensionMapping) -> Self {
        Self {
            schema,
            mapping,
            categorical_encoders: HashMap::new(),
            predicates: Vec::new(),
        }
    }

    /// Registers the dictionary encoder for a categorical field.
    pub fn with_categorical_encoder(
        mut self,
        field: usize,
        encoder: CategoricalDictionaryEncoder,
    ) -> Self {
        self.categorical_encoders.insert(field, encoder);
        self
    }

    /// Registers categorical encoders from record encoder metadata.
    pub fn try_with_record_encoder_metadata(
        mut self,
        metadata: &FSERecordEncoderMetadata,
    ) -> Result<Self, TypedQueryPlanError> {
        let _encoder = metadata.to_record_encoder(self.schema)?;

        for (field, metadata) in metadata.fields().iter().enumerate() {
            let FSEFieldEncoderMetadata::CategoryDictionary { categories } = metadata else {
                continue;
            };

            self.categorical_encoders
                .insert(field, CategoricalDictionaryEncoder::new(categories.clone()));
        }

        Ok(self)
    }

    /// Adds a predicate to the builder.
    pub fn with_predicate(mut self, predicate: FSEPredicate) -> Self {
        self.predicates.push(predicate);
        self
    }

    /// Adds a predicate to the builder in place.
    pub fn push_predicate(&mut self, predicate: FSEPredicate) -> &mut Self {
        self.predicates.push(predicate);
        self
    }

    /// Builds a conjunctive typed query plan.
    pub fn build(self) -> Result<TypedQueryPlan, TypedQueryPlanError> {
        let mut plans = Vec::with_capacity(self.predicates.len());

        for predicate in self.predicates {
            let predicate = predicate.validate(self.schema)?;
            let query_region = match predicate.field_type() {
                FSEFieldType::Integer | FSEFieldType::Float | FSEFieldType::TimestampMillis => {
                    compile_numeric_predicate_to_query_region(&predicate, self.mapping)?
                }
                FSEFieldType::Category => {
                    let Some(encoder) = self.categorical_encoders.get(&predicate.field()) else {
                        return Err(TypedQueryPlanError::MissingCategoricalEncoder {
                            field: predicate.field(),
                            name: predicate.name().to_string(),
                        });
                    };

                    compile_categorical_equality_predicate_to_query_region(
                        &predicate,
                        self.mapping,
                        encoder,
                    )?
                }
                field_type => {
                    return Err(FSEPredicateCompileError::UnsupportedFieldType {
                        field: predicate.field(),
                        name: predicate.name().to_string(),
                        field_type,
                    }
                    .into());
                }
            };

            plans.push(TypedQueryPlan::new(predicate, query_region));
        }

        TypedQueryPlan::conjunctive(plans)
    }
}
