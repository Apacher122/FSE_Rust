//! Semantics-preserving encoders.
//!
//! This module defines encoder contracts for mapping FSE-native typed data into
//! numeric coordinate space. Concrete encoders are added separately.

mod batch;
mod categorical;
mod coordinate;
mod error;
mod metadata;
mod numeric;
mod record;
mod traits;

pub use batch::{EncodedRecordBatch, FSERecordBatchEncodingError, encode_record_batch};
pub use categorical::{CategoricalDictionaryEncoder, CategoricalDictionaryError};
pub use coordinate::EncodedCoordinates;
pub use error::FSEEncodingError;
pub use metadata::{
    FSEFieldEncoderMetadata, FSERecordEncoderMetadata, FSERecordEncoderMetadataError,
};
pub use numeric::{BooleanEncoder, FloatEncoder, IntegerEncoder, TimestampMillisEncoder};
pub use record::{
    ComposedRecordEncoder, ComposedRecordEncoderError, ComposedRecordEncoderFromBatchError,
};
pub use traits::{FSEFieldEncoder, FSERecordEncoder};
