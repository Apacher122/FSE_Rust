//! Query region representation.

mod classification;
mod evaluator;
mod query_region;

pub use classification::QueryBoundsClassification;
pub use evaluator::evaluate_query;
pub use query_region::{QueryRegion, QueryRegionError};
