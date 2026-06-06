//! Typed query planning and execution.

pub mod builder;
pub mod compiler;
pub mod evaluator;
pub mod execution;
pub mod index;
pub mod plan;
pub mod predicate;

pub use builder::TypedQueryPlanBuilder;
pub use compiler::{
    FSEPredicateCompileError, compile_categorical_equality_predicate_to_query_region,
    compile_numeric_predicate_to_query_region,
};
pub use evaluator::evaluate_typed_predicate;
pub use execution::{
    IndexedTypedQueryError, IndexedTypedQueryReport, IndexedTypedQueryRowReport,
    TypedQueryResultRow, evaluate_indexed_typed_query_plan, evaluate_indexed_typed_query_plan_rows,
    evaluate_indexed_typed_query_plan_rows_with_stats,
    evaluate_indexed_typed_query_plan_with_stats, evaluate_typed_query_plan,
    evaluate_typed_query_plan_rows,
};
pub use index::{TypedQueryIndex, TypedQueryIndexBuildError};
pub use plan::{TypedQueryPlan, TypedQueryPlanError};
pub use predicate::{
    FSEPredicate, FSEPredicateError, FSEPredicateField, FSEPredicateOperator,
    ValidatedFSEPredicate, ValidatedFSEPredicateOperator,
};
