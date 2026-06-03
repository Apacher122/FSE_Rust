//! FSE-native typed data.
//!
//! This module defines typed logical values and schema metadata used before
//! semantic encoding maps records into numeric coordinate space.

mod record;
mod schema;
mod value;

pub use record::{FSERecord, FSERecordError};
pub use schema::{FSEField, FSESchema, FSESchemaError};
pub use value::{FSEFieldType, FSEValue};
