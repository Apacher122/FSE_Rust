//! Binary encoding for typed query index archive snapshots.

use std::error::Error;
use std::fmt;
use std::mem::size_of;
use std::string::FromUtf8Error;

use crate::encoding::{
    FSEFieldEncoderMetadata, FSERecordEncoderMetadata, FSERecordEncoderMetadataError,
};
use crate::persistence::{
    FSERowMappedArchiveCodecError, FSETypedRecordBatchArchiveCodecError,
    decode_row_mapped_archive_snapshot, decode_typed_record_batch_archive_snapshot,
    encode_row_mapped_archive_snapshot, encode_typed_record_batch_archive_snapshot,
};

use super::{FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveSnapshotError};

/// Error returned when typed query index archive byte encoding or decoding fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveCodecError {
    /// The typed query index snapshot failed validation.
    Snapshot(FSETypedQueryIndexArchiveSnapshotError),

    /// The embedded row-mapped index snapshot codec failed.
    IndexCodec(FSERowMappedArchiveCodecError),

    /// The embedded typed record batch snapshot codec failed.
    BatchCodec(FSETypedRecordBatchArchiveCodecError),

    /// The embedded record encoder metadata failed validation.
    EncoderMetadata(FSERecordEncoderMetadataError),

    /// The byte slice ended before a complete typed query index field could be read.
    UnexpectedEndOfArchive {
        /// Archive field being read.
        field: &'static str,
        /// Number of bytes required for the field.
        needed: usize,
        /// Number of bytes remaining in the input.
        remaining: usize,
    },

    /// The archive contained bytes after the decoded typed query index snapshot.
    TrailingBytes {
        /// Number of unread bytes.
        remaining: usize,
    },

    /// A length field cannot be represented by this runtime.
    LengthOutOfRange {
        /// Archive field being read.
        field: &'static str,
        /// Length found in the input.
        length: u64,
    },

    /// A field encoder metadata tag was not recognized.
    InvalidFieldEncoderMetadataTag {
        /// Archive field being read.
        field: &'static str,
        /// Tag found in the input.
        tag: u8,
    },

    /// A UTF-8 string field could not be decoded.
    InvalidUtf8 {
        /// Archive field being read.
        field: &'static str,
    },
}

impl fmt::Display for FSETypedQueryIndexArchiveCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::IndexCodec(error) => error.fmt(formatter),
            Self::BatchCodec(error) => error.fmt(formatter),
            Self::EncoderMetadata(error) => error.fmt(formatter),
            Self::UnexpectedEndOfArchive { .. } => formatter
                .write_str("typed query index archive ended before the field could be read"),
            Self::TrailingBytes { .. } => {
                formatter.write_str("typed query index archive contains trailing bytes")
            }
            Self::LengthOutOfRange { .. } => formatter
                .write_str("typed query index archive length field is outside the runtime range"),
            Self::InvalidFieldEncoderMetadataTag { .. } => formatter
                .write_str("typed query index archive contains an invalid encoder metadata tag"),
            Self::InvalidUtf8 { .. } => {
                formatter.write_str("typed query index archive contains invalid UTF-8")
            }
        }
    }
}

impl Error for FSETypedQueryIndexArchiveCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::IndexCodec(error) => Some(error),
            Self::BatchCodec(error) => Some(error),
            Self::EncoderMetadata(error) => Some(error),
            Self::UnexpectedEndOfArchive { .. }
            | Self::TrailingBytes { .. }
            | Self::LengthOutOfRange { .. }
            | Self::InvalidFieldEncoderMetadataTag { .. }
            | Self::InvalidUtf8 { .. } => None,
        }
    }
}

impl From<FSETypedQueryIndexArchiveSnapshotError> for FSETypedQueryIndexArchiveCodecError {
    fn from(error: FSETypedQueryIndexArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<FSERowMappedArchiveCodecError> for FSETypedQueryIndexArchiveCodecError {
    fn from(error: FSERowMappedArchiveCodecError) -> Self {
        Self::IndexCodec(error)
    }
}

impl From<FSETypedRecordBatchArchiveCodecError> for FSETypedQueryIndexArchiveCodecError {
    fn from(error: FSETypedRecordBatchArchiveCodecError) -> Self {
        Self::BatchCodec(error)
    }
}

impl From<FSERecordEncoderMetadataError> for FSETypedQueryIndexArchiveCodecError {
    fn from(error: FSERecordEncoderMetadataError) -> Self {
        Self::EncoderMetadata(error)
    }
}

/// Encodes a typed query index archive snapshot into little-endian bytes.
pub fn encode_typed_query_index_archive_snapshot(
    snapshot: &FSETypedQueryIndexArchiveSnapshot,
) -> Result<Vec<u8>, FSETypedQueryIndexArchiveCodecError> {
    snapshot.validate()?;

    let index_bytes = encode_row_mapped_archive_snapshot(&snapshot.index)
        .map_err(FSETypedQueryIndexArchiveCodecError::IndexCodec)?;
    let batch_bytes = encode_typed_record_batch_archive_snapshot(&snapshot.batch)
        .map_err(FSETypedQueryIndexArchiveCodecError::BatchCodec)?;
    let record_encoder_bytes = encode_record_encoder_metadata(&snapshot.record_encoder);
    let mut bytes = Vec::new();

    write_byte_vec(&mut bytes, &index_bytes);
    write_byte_vec(&mut bytes, &batch_bytes);
    write_byte_vec(&mut bytes, &record_encoder_bytes);

    Ok(bytes)
}

/// Decodes a typed query index archive snapshot from little-endian bytes.
pub fn decode_typed_query_index_archive_snapshot(
    bytes: &[u8],
) -> Result<FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveCodecError> {
    let mut reader = TypedQueryIndexArchiveReader::new(bytes);
    let index_bytes = reader.read_byte_vec("typed_index.row_mapped_index")?;
    let batch_bytes = reader.read_byte_vec("typed_index.record_batch")?;
    let record_encoder_bytes = reader.read_byte_vec("typed_index.record_encoder")?;

    if reader.remaining() != 0 {
        return Err(FSETypedQueryIndexArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let index = decode_row_mapped_archive_snapshot(&index_bytes)
        .map_err(FSETypedQueryIndexArchiveCodecError::IndexCodec)?;
    let batch = decode_typed_record_batch_archive_snapshot(&batch_bytes)
        .map_err(FSETypedQueryIndexArchiveCodecError::BatchCodec)?;
    let record_encoder = decode_record_encoder_metadata(&record_encoder_bytes)?;
    let snapshot = FSETypedQueryIndexArchiveSnapshot {
        index,
        batch,
        record_encoder,
    };
    snapshot.validate()?;

    Ok(snapshot)
}

impl FSETypedQueryIndexArchiveSnapshot {
    /// Encodes this typed query index snapshot into little-endian archive bytes.
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, FSETypedQueryIndexArchiveCodecError> {
        encode_typed_query_index_archive_snapshot(self)
    }

    /// Decodes a typed query index snapshot from little-endian archive bytes.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, FSETypedQueryIndexArchiveCodecError> {
        decode_typed_query_index_archive_snapshot(bytes)
    }
}

fn write_byte_vec(bytes: &mut Vec<u8>, values: &[u8]) {
    write_u64(bytes, values.len() as u64);
    bytes.extend_from_slice(values);
}

fn encode_record_encoder_metadata(metadata: &FSERecordEncoderMetadata) -> Vec<u8> {
    let mut bytes = Vec::new();

    write_u64(&mut bytes, metadata.fields().len() as u64);

    for field in metadata.fields() {
        encode_field_encoder_metadata(&mut bytes, field);
    }

    bytes
}

fn encode_field_encoder_metadata(bytes: &mut Vec<u8>, metadata: &FSEFieldEncoderMetadata) {
    match metadata {
        FSEFieldEncoderMetadata::Integer => bytes.push(FIELD_ENCODER_INTEGER_TAG),
        FSEFieldEncoderMetadata::Float => bytes.push(FIELD_ENCODER_FLOAT_TAG),
        FSEFieldEncoderMetadata::Boolean => bytes.push(FIELD_ENCODER_BOOLEAN_TAG),
        FSEFieldEncoderMetadata::TimestampMillis => bytes.push(FIELD_ENCODER_TIMESTAMP_MILLIS_TAG),
        FSEFieldEncoderMetadata::CategoryDictionary { categories } => {
            bytes.push(FIELD_ENCODER_CATEGORY_DICTIONARY_TAG);
            write_u64(bytes, categories.len() as u64);

            for category in categories {
                write_string(bytes, category);
            }
        }
    }
}

fn decode_record_encoder_metadata(
    bytes: &[u8],
) -> Result<FSERecordEncoderMetadata, FSETypedQueryIndexArchiveCodecError> {
    let mut reader = TypedQueryIndexArchiveReader::new(bytes);
    let field_count = reader.read_len("typed_index.record_encoder.field_count")?;
    let mut fields = Vec::with_capacity(field_count);

    for _ in 0..field_count {
        fields.push(decode_field_encoder_metadata(&mut reader)?);
    }

    if reader.remaining() != 0 {
        return Err(FSETypedQueryIndexArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    Ok(FSERecordEncoderMetadata::new(fields))
}

fn decode_field_encoder_metadata(
    reader: &mut TypedQueryIndexArchiveReader<'_>,
) -> Result<FSEFieldEncoderMetadata, FSETypedQueryIndexArchiveCodecError> {
    let field = "typed_index.record_encoder.field";
    let tag = reader.read_u8(field)?;

    match tag {
        FIELD_ENCODER_INTEGER_TAG => Ok(FSEFieldEncoderMetadata::Integer),
        FIELD_ENCODER_FLOAT_TAG => Ok(FSEFieldEncoderMetadata::Float),
        FIELD_ENCODER_BOOLEAN_TAG => Ok(FSEFieldEncoderMetadata::Boolean),
        FIELD_ENCODER_TIMESTAMP_MILLIS_TAG => Ok(FSEFieldEncoderMetadata::TimestampMillis),
        FIELD_ENCODER_CATEGORY_DICTIONARY_TAG => {
            let category_count = reader.read_len("typed_index.record_encoder.category_count")?;
            let mut categories = Vec::with_capacity(category_count);

            for _ in 0..category_count {
                categories.push(reader.read_string("typed_index.record_encoder.category")?);
            }

            Ok(FSEFieldEncoderMetadata::CategoryDictionary { categories })
        }
        tag => {
            Err(FSETypedQueryIndexArchiveCodecError::InvalidFieldEncoderMetadataTag { field, tag })
        }
    }
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_byte_vec(bytes, value.as_bytes());
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

const FIELD_ENCODER_INTEGER_TAG: u8 = 0;
const FIELD_ENCODER_FLOAT_TAG: u8 = 1;
const FIELD_ENCODER_BOOLEAN_TAG: u8 = 2;
const FIELD_ENCODER_TIMESTAMP_MILLIS_TAG: u8 = 3;
const FIELD_ENCODER_CATEGORY_DICTIONARY_TAG: u8 = 4;

struct TypedQueryIndexArchiveReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> TypedQueryIndexArchiveReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_byte_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u8>, FSETypedQueryIndexArchiveCodecError> {
        let length = self.read_len(field)?;
        let bytes = self.read_exact(field, length)?;

        Ok(bytes.to_vec())
    }

    fn read_string(
        &mut self,
        field: &'static str,
    ) -> Result<String, FSETypedQueryIndexArchiveCodecError> {
        let bytes = self.read_byte_vec(field)?;

        String::from_utf8(bytes).map_err(|_source: FromUtf8Error| {
            FSETypedQueryIndexArchiveCodecError::InvalidUtf8 { field }
        })
    }

    fn read_len(
        &mut self,
        field: &'static str,
    ) -> Result<usize, FSETypedQueryIndexArchiveCodecError> {
        let length = self.read_u64(field)?;

        usize::try_from(length)
            .map_err(|_| FSETypedQueryIndexArchiveCodecError::LengthOutOfRange { field, length })
    }

    fn read_u64(
        &mut self,
        field: &'static str,
    ) -> Result<u64, FSETypedQueryIndexArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<u64>())?;

        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_u8(&mut self, field: &'static str) -> Result<u8, FSETypedQueryIndexArchiveCodecError> {
        let bytes = self.read_exact(field, 1)?;

        Ok(bytes[0])
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<&'a [u8], FSETypedQueryIndexArchiveCodecError> {
        let remaining = self.remaining();
        let Some(end) = self.offset.checked_add(length) else {
            return Err(
                FSETypedQueryIndexArchiveCodecError::UnexpectedEndOfArchive {
                    field,
                    needed: length,
                    remaining,
                },
            );
        };

        if end > self.bytes.len() {
            return Err(
                FSETypedQueryIndexArchiveCodecError::UnexpectedEndOfArchive {
                    field,
                    needed: length,
                    remaining,
                },
            );
        }

        let slice = &self.bytes[self.offset..end];
        self.offset = end;

        Ok(slice)
    }
}
