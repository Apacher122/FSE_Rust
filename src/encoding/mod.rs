//! Semantics-preserving encoders.
//!
//! This module defines encoder contracts for mapping FSE-native typed data into
//! numeric coordinate space. Concrete encoders are added separately.

mod coordinate;
mod error;
mod numeric;
mod traits;

pub use coordinate::EncodedCoordinates;
pub use error::FSEEncodingError;
pub use numeric::{BooleanEncoder, FloatEncoder, IntegerEncoder, TimestampMillisEncoder};
pub use traits::{FSEFieldEncoder, FSERecordEncoder};
