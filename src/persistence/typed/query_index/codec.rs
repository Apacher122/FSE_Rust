//! Binary encoding for typed query index archive snapshots.

use std::error::Error;
use std::fmt;
use std::mem::size_of;
use std::string::FromUtf8Error;

use crate::encoding::{
    FSEFieldEncoderMetadata, FSERecordEncoderMetadata, FSERecordEncoderMetadataError,
};
use crate::persistence::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSERowMappedArchiveCodecError,
    FSETypedRecordBatchArchiveCodecError, FSEValueArchiveRecord,
    decode_row_mapped_archive_snapshot, decode_typed_record_batch_archive_snapshot,
    encode_row_mapped_archive_snapshot,
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

    /// The compact typed record batch field count did not match encoder metadata.
    CompactBatchEncoderFieldCountMismatch {
        /// Number of schema fields stored in the record batch section.
        schema_field_count: usize,

        /// Number of field encoders stored in metadata.
        encoder_field_count: usize,
    },

    /// A categorical value was not described by categorical encoder metadata.
    CompactBatchCategoryEncoderMismatch {
        /// Field index containing the categorical value.
        field_index: usize,
    },

    /// A categorical value was not present in encoder metadata.
    CompactBatchCategoryNotInEncoderMetadata {
        /// Field index containing the categorical value.
        field_index: usize,

        /// Category label found in the record batch.
        category: String,
    },

    /// A categorical code was outside the encoder dictionary.
    CompactBatchCategoryCodeOutOfRange {
        /// Field index containing the categorical value.
        field_index: usize,

        /// Encoded category code found in the archive.
        code: u64,

        /// Number of categories available in encoder metadata.
        category_count: usize,
    },

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

    /// A compact typed record batch field type tag was not recognized.
    InvalidCompactBatchFieldTypeTag {
        /// Archive field being read.
        field: &'static str,
        /// Tag found in the input.
        tag: u8,
    },

    /// A compact typed record batch value tag was not recognized.
    InvalidCompactBatchValueTag {
        /// Archive field being read.
        field: &'static str,
        /// Tag found in the input.
        tag: u8,
    },

    /// A compact typed record batch boolean field contained an invalid byte.
    InvalidCompactBatchBoolean {
        /// Archive field being read.
        field: &'static str,
        /// Raw boolean value found in the archive.
        value: u8,
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
            Self::CompactBatchEncoderFieldCountMismatch { .. } => formatter.write_str(
                "compact typed record batch field count does not match encoder metadata",
            ),
            Self::CompactBatchCategoryEncoderMismatch { .. } => formatter.write_str(
                "compact typed record batch category value requires category encoder metadata",
            ),
            Self::CompactBatchCategoryNotInEncoderMetadata { .. } => formatter
                .write_str("compact typed record batch category is missing from encoder metadata"),
            Self::CompactBatchCategoryCodeOutOfRange { .. } => formatter
                .write_str("compact typed record batch category code is outside encoder metadata"),
            Self::UnexpectedEndOfArchive { .. } => formatter
                .write_str("typed query index archive ended before the field could be read"),
            Self::TrailingBytes { .. } => {
                formatter.write_str("typed query index archive contains trailing bytes")
            }
            Self::LengthOutOfRange { .. } => formatter
                .write_str("typed query index archive length field is outside the runtime range"),
            Self::InvalidFieldEncoderMetadataTag { .. } => formatter
                .write_str("typed query index archive contains an invalid encoder metadata tag"),
            Self::InvalidCompactBatchFieldTypeTag { .. } => formatter
                .write_str("compact typed record batch archive contains an invalid field type tag"),
            Self::InvalidCompactBatchValueTag { .. } => formatter
                .write_str("compact typed record batch archive contains an invalid value tag"),
            Self::InvalidCompactBatchBoolean { .. } => {
                formatter.write_str("compact typed record batch archive boolean field is invalid")
            }
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
            Self::CompactBatchEncoderFieldCountMismatch { .. }
            | Self::CompactBatchCategoryEncoderMismatch { .. }
            | Self::CompactBatchCategoryNotInEncoderMetadata { .. }
            | Self::CompactBatchCategoryCodeOutOfRange { .. }
            | Self::InvalidCompactBatchFieldTypeTag { .. }
            | Self::InvalidCompactBatchValueTag { .. }
            | Self::InvalidCompactBatchBoolean { .. }
            | Self::UnexpectedEndOfArchive { .. }
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
    let batch_bytes =
        encode_typed_query_index_record_batch_section(&snapshot.batch, &snapshot.record_encoder)?;
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
    let record_encoder = decode_record_encoder_metadata(&record_encoder_bytes)?;
    let batch = decode_typed_query_index_record_batch_section(&batch_bytes, &record_encoder)?;
    let snapshot = FSETypedQueryIndexArchiveSnapshot {
        index,
        batch,
        record_encoder,
    };
    snapshot.validate()?;

    Ok(snapshot)
}

pub(super) fn encode_typed_query_index_record_batch_section(
    snapshot: &FSERecordBatchArchiveSnapshot,
    record_encoder: &FSERecordEncoderMetadata,
) -> Result<Vec<u8>, FSETypedQueryIndexArchiveCodecError> {
    snapshot
        .validate()
        .map_err(FSETypedRecordBatchArchiveCodecError::Snapshot)
        .map_err(FSETypedQueryIndexArchiveCodecError::BatchCodec)?;

    validate_compact_batch_encoder_field_count(snapshot, record_encoder)?;

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&COMPACT_TYPED_BATCH_SECTION_MAGIC);

    write_u64(&mut bytes, snapshot.schema_fields.len() as u64);
    for field in &snapshot.schema_fields {
        write_compact_batch_field(&mut bytes, field);
    }

    write_u64_vec(&mut bytes, &snapshot.row_ids);

    write_u64(&mut bytes, snapshot.records.len() as u64);
    for record in &snapshot.records {
        write_compact_batch_record(&mut bytes, record, record_encoder.fields())?;
    }

    Ok(bytes)
}

fn decode_typed_query_index_record_batch_section(
    bytes: &[u8],
    record_encoder: &FSERecordEncoderMetadata,
) -> Result<FSERecordBatchArchiveSnapshot, FSETypedQueryIndexArchiveCodecError> {
    if !bytes.starts_with(&COMPACT_TYPED_BATCH_SECTION_MAGIC) {
        return decode_typed_record_batch_archive_snapshot(bytes)
            .map_err(FSETypedQueryIndexArchiveCodecError::BatchCodec);
    }

    let mut reader = TypedQueryIndexArchiveReader::new(bytes);
    reader.read_exact(
        "typed_index.record_batch.compact_magic",
        COMPACT_TYPED_BATCH_SECTION_MAGIC.len(),
    )?;

    let field_count = reader.read_len("typed_index.record_batch.schema.field_count")?;
    if field_count != record_encoder.fields().len() {
        return Err(
            FSETypedQueryIndexArchiveCodecError::CompactBatchEncoderFieldCountMismatch {
                schema_field_count: field_count,
                encoder_field_count: record_encoder.fields().len(),
            },
        );
    }

    let mut schema_fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        schema_fields.push(reader.read_compact_batch_field()?);
    }

    let row_ids = reader.read_u64_vec("typed_index.record_batch.row_ids")?;
    let record_count = reader.read_len("typed_index.record_batch.record_count")?;
    let mut records = Vec::with_capacity(record_count);

    for _ in 0..record_count {
        records.push(reader.read_compact_batch_record(record_encoder.fields())?);
    }

    if reader.remaining() != 0 {
        return Err(FSETypedQueryIndexArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let snapshot = FSERecordBatchArchiveSnapshot {
        schema_fields,
        row_ids,
        records,
    };
    snapshot
        .validate()
        .map_err(FSETypedRecordBatchArchiveCodecError::Snapshot)
        .map_err(FSETypedQueryIndexArchiveCodecError::BatchCodec)?;

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

fn validate_compact_batch_encoder_field_count(
    snapshot: &FSERecordBatchArchiveSnapshot,
    record_encoder: &FSERecordEncoderMetadata,
) -> Result<(), FSETypedQueryIndexArchiveCodecError> {
    if snapshot.schema_fields.len() == record_encoder.fields().len() {
        return Ok(());
    }

    Err(
        FSETypedQueryIndexArchiveCodecError::CompactBatchEncoderFieldCountMismatch {
            schema_field_count: snapshot.schema_fields.len(),
            encoder_field_count: record_encoder.fields().len(),
        },
    )
}

fn write_compact_batch_field(bytes: &mut Vec<u8>, field: &FSEFieldArchiveRecord) {
    write_string(bytes, &field.name);
    write_compact_batch_field_type_tag(bytes, field.field_type);
    write_bool(bytes, field.nullable);
}

fn write_compact_batch_record(
    bytes: &mut Vec<u8>,
    record: &FSERecordArchiveRecord,
    fields: &[FSEFieldEncoderMetadata],
) -> Result<(), FSETypedQueryIndexArchiveCodecError> {
    write_u64(bytes, record.values.len() as u64);

    for (field_index, value) in record.values.iter().enumerate() {
        let field_encoder = fields.get(field_index).ok_or(
            FSETypedQueryIndexArchiveCodecError::CompactBatchEncoderFieldCountMismatch {
                schema_field_count: record.values.len(),
                encoder_field_count: fields.len(),
            },
        )?;
        write_compact_batch_value(bytes, field_index, value, field_encoder)?;
    }

    Ok(())
}

fn write_compact_batch_value(
    bytes: &mut Vec<u8>,
    field_index: usize,
    value: &FSEValueArchiveRecord,
    field_encoder: &FSEFieldEncoderMetadata,
) -> Result<(), FSETypedQueryIndexArchiveCodecError> {
    match value {
        FSEValueArchiveRecord::Null => {
            bytes.push(COMPACT_BATCH_VALUE_NULL_TAG);
        }
        FSEValueArchiveRecord::Integer(value) => {
            bytes.push(COMPACT_BATCH_VALUE_INTEGER_TAG);
            write_i64(bytes, *value);
        }
        FSEValueArchiveRecord::Float(value) => {
            bytes.push(COMPACT_BATCH_VALUE_FLOAT_TAG);
            write_f64(bytes, *value);
        }
        FSEValueArchiveRecord::Text(value) => {
            bytes.push(COMPACT_BATCH_VALUE_TEXT_TAG);
            write_string(bytes, value);
        }
        FSEValueArchiveRecord::Boolean(value) => {
            bytes.push(COMPACT_BATCH_VALUE_BOOLEAN_TAG);
            write_bool(bytes, *value);
        }
        FSEValueArchiveRecord::TimestampMillis(value) => {
            bytes.push(COMPACT_BATCH_VALUE_TIMESTAMP_MILLIS_TAG);
            write_i64(bytes, *value);
        }
        FSEValueArchiveRecord::Category(value) => {
            let FSEFieldEncoderMetadata::CategoryDictionary { categories } = field_encoder else {
                return Err(
                    FSETypedQueryIndexArchiveCodecError::CompactBatchCategoryEncoderMismatch {
                        field_index,
                    },
                );
            };
            let code = categories
                .iter()
                .position(|category| category == value)
                .ok_or_else(|| {
                    FSETypedQueryIndexArchiveCodecError::CompactBatchCategoryNotInEncoderMetadata {
                        field_index,
                        category: value.clone(),
                    }
                })?;

            bytes.push(COMPACT_BATCH_VALUE_CATEGORY_TAG);
            write_u64(bytes, code as u64);
        }
    }

    Ok(())
}

fn write_compact_batch_field_type_tag(bytes: &mut Vec<u8>, tag: FSEFieldTypeArchiveTag) {
    bytes.push(match tag {
        FSEFieldTypeArchiveTag::Integer => COMPACT_BATCH_FIELD_INTEGER_TAG,
        FSEFieldTypeArchiveTag::Float => COMPACT_BATCH_FIELD_FLOAT_TAG,
        FSEFieldTypeArchiveTag::Text => COMPACT_BATCH_FIELD_TEXT_TAG,
        FSEFieldTypeArchiveTag::Boolean => COMPACT_BATCH_FIELD_BOOLEAN_TAG,
        FSEFieldTypeArchiveTag::TimestampMillis => COMPACT_BATCH_FIELD_TIMESTAMP_MILLIS_TAG,
        FSEFieldTypeArchiveTag::Category => COMPACT_BATCH_FIELD_CATEGORY_TAG,
    });
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

fn write_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    write_u64(bytes, values.len() as u64);

    for value in values {
        write_u64(bytes, *value);
    }
}

fn write_bool(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}

fn write_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

const COMPACT_TYPED_BATCH_SECTION_MAGIC: [u8; 8] = *b"FSECBT01";

const COMPACT_BATCH_FIELD_INTEGER_TAG: u8 = 0;
const COMPACT_BATCH_FIELD_FLOAT_TAG: u8 = 1;
const COMPACT_BATCH_FIELD_TEXT_TAG: u8 = 2;
const COMPACT_BATCH_FIELD_BOOLEAN_TAG: u8 = 3;
const COMPACT_BATCH_FIELD_TIMESTAMP_MILLIS_TAG: u8 = 4;
const COMPACT_BATCH_FIELD_CATEGORY_TAG: u8 = 5;

const COMPACT_BATCH_VALUE_NULL_TAG: u8 = 0;
const COMPACT_BATCH_VALUE_INTEGER_TAG: u8 = 1;
const COMPACT_BATCH_VALUE_FLOAT_TAG: u8 = 2;
const COMPACT_BATCH_VALUE_TEXT_TAG: u8 = 3;
const COMPACT_BATCH_VALUE_BOOLEAN_TAG: u8 = 4;
const COMPACT_BATCH_VALUE_TIMESTAMP_MILLIS_TAG: u8 = 5;
const COMPACT_BATCH_VALUE_CATEGORY_TAG: u8 = 6;

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

    fn read_compact_batch_field(
        &mut self,
    ) -> Result<FSEFieldArchiveRecord, FSETypedQueryIndexArchiveCodecError> {
        Ok(FSEFieldArchiveRecord {
            name: self.read_string("typed_index.record_batch.schema.field.name")?,
            field_type: self
                .read_compact_batch_field_type_tag("typed_index.record_batch.schema.field.type")?,
            nullable: self.read_bool("typed_index.record_batch.schema.field.nullable")?,
        })
    }

    fn read_compact_batch_record(
        &mut self,
        fields: &[FSEFieldEncoderMetadata],
    ) -> Result<FSERecordArchiveRecord, FSETypedQueryIndexArchiveCodecError> {
        let value_count = self.read_len("typed_index.record_batch.record.value_count")?;
        let mut values = Vec::with_capacity(value_count);

        for field_index in 0..value_count {
            values.push(self.read_compact_batch_value(field_index, fields)?);
        }

        Ok(FSERecordArchiveRecord { values })
    }

    fn read_compact_batch_value(
        &mut self,
        field_index: usize,
        fields: &[FSEFieldEncoderMetadata],
    ) -> Result<FSEValueArchiveRecord, FSETypedQueryIndexArchiveCodecError> {
        let field = "typed_index.record_batch.record.value";

        match self.read_u8(field)? {
            COMPACT_BATCH_VALUE_NULL_TAG => Ok(FSEValueArchiveRecord::Null),
            COMPACT_BATCH_VALUE_INTEGER_TAG => {
                Ok(FSEValueArchiveRecord::Integer(self.read_i64(field)?))
            }
            COMPACT_BATCH_VALUE_FLOAT_TAG => {
                Ok(FSEValueArchiveRecord::Float(self.read_f64(field)?))
            }
            COMPACT_BATCH_VALUE_TEXT_TAG => {
                Ok(FSEValueArchiveRecord::Text(self.read_string(field)?))
            }
            COMPACT_BATCH_VALUE_BOOLEAN_TAG => {
                Ok(FSEValueArchiveRecord::Boolean(self.read_bool(field)?))
            }
            COMPACT_BATCH_VALUE_TIMESTAMP_MILLIS_TAG => Ok(FSEValueArchiveRecord::TimestampMillis(
                self.read_i64(field)?,
            )),
            COMPACT_BATCH_VALUE_CATEGORY_TAG => {
                let code = self.read_u64(field)?;
                let Some(FSEFieldEncoderMetadata::CategoryDictionary { categories }) =
                    fields.get(field_index)
                else {
                    return Err(
                        FSETypedQueryIndexArchiveCodecError::CompactBatchCategoryEncoderMismatch {
                            field_index,
                        },
                    );
                };
                let code_index = usize::try_from(code).map_err(|_| {
                    FSETypedQueryIndexArchiveCodecError::CompactBatchCategoryCodeOutOfRange {
                        field_index,
                        code,
                        category_count: categories.len(),
                    }
                })?;
                let Some(category) = categories.get(code_index) else {
                    return Err(
                        FSETypedQueryIndexArchiveCodecError::CompactBatchCategoryCodeOutOfRange {
                            field_index,
                            code,
                            category_count: categories.len(),
                        },
                    );
                };

                Ok(FSEValueArchiveRecord::Category(category.clone()))
            }
            tag => {
                Err(FSETypedQueryIndexArchiveCodecError::InvalidCompactBatchValueTag { field, tag })
            }
        }
    }

    fn read_compact_batch_field_type_tag(
        &mut self,
        field: &'static str,
    ) -> Result<FSEFieldTypeArchiveTag, FSETypedQueryIndexArchiveCodecError> {
        match self.read_u8(field)? {
            COMPACT_BATCH_FIELD_INTEGER_TAG => Ok(FSEFieldTypeArchiveTag::Integer),
            COMPACT_BATCH_FIELD_FLOAT_TAG => Ok(FSEFieldTypeArchiveTag::Float),
            COMPACT_BATCH_FIELD_TEXT_TAG => Ok(FSEFieldTypeArchiveTag::Text),
            COMPACT_BATCH_FIELD_BOOLEAN_TAG => Ok(FSEFieldTypeArchiveTag::Boolean),
            COMPACT_BATCH_FIELD_TIMESTAMP_MILLIS_TAG => Ok(FSEFieldTypeArchiveTag::TimestampMillis),
            COMPACT_BATCH_FIELD_CATEGORY_TAG => Ok(FSEFieldTypeArchiveTag::Category),
            tag => Err(
                FSETypedQueryIndexArchiveCodecError::InvalidCompactBatchFieldTypeTag { field, tag },
            ),
        }
    }

    fn read_u64_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u64>, FSETypedQueryIndexArchiveCodecError> {
        let length = self.read_len(field)?;
        let mut values = Vec::with_capacity(length);

        for _ in 0..length {
            values.push(self.read_u64(field)?);
        }

        Ok(values)
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

    fn read_bool(
        &mut self,
        field: &'static str,
    ) -> Result<bool, FSETypedQueryIndexArchiveCodecError> {
        let value = self.read_u8(field)?;

        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(
                FSETypedQueryIndexArchiveCodecError::InvalidCompactBatchBoolean { field, value },
            ),
        }
    }

    fn read_i64(
        &mut self,
        field: &'static str,
    ) -> Result<i64, FSETypedQueryIndexArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<i64>())?;

        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_f64(
        &mut self,
        field: &'static str,
    ) -> Result<f64, FSETypedQueryIndexArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<f64>())?;

        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
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
