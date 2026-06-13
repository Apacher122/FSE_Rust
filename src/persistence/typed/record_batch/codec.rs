//! Binary encoding for typed record batch archive snapshots.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use super::{
    FSEFieldArchiveRecord, FSEFieldTypeArchiveTag, FSERecordArchiveRecord,
    FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveSnapshotError, FSEValueArchiveRecord,
};

/// Error returned when typed record batch archive byte encoding or decoding fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedRecordBatchArchiveCodecError {
    /// The typed record batch snapshot failed validation.
    Snapshot(FSETypedRecordBatchArchiveSnapshotError),

    /// The byte slice ended before a complete typed record batch field could be read.
    UnexpectedEndOfArchive {
        /// Archive field being read.
        field: &'static str,
        /// Number of bytes required for the field.
        needed: usize,
        /// Number of bytes remaining in the input.
        remaining: usize,
    },

    /// A string field was not valid UTF-8.
    InvalidUtf8 {
        /// Archive field being read.
        field: &'static str,
        /// Raw bytes found for the field.
        bytes: Vec<u8>,
    },

    /// A boolean field contained a value other than `0` or `1`.
    InvalidBoolean {
        /// Archive field being read.
        field: &'static str,
        /// Raw boolean value found in the input.
        value: u8,
    },

    /// A field type tag did not match a supported field type.
    InvalidFieldTypeTag {
        /// Archive field being read.
        field: &'static str,
        /// Raw tag value found in the input.
        value: u8,
    },

    /// A value tag did not match a supported typed value variant.
    InvalidValueTag {
        /// Archive field being read.
        field: &'static str,
        /// Raw tag value found in the input.
        value: u8,
    },

    /// The archive contained bytes after the decoded typed record batch snapshot.
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
}

impl fmt::Display for FSETypedRecordBatchArchiveCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::UnexpectedEndOfArchive { .. } => formatter
                .write_str("typed record batch archive ended before the field could be read"),
            Self::InvalidUtf8 { .. } => {
                formatter.write_str("typed record batch archive string field is not valid UTF-8")
            }
            Self::InvalidBoolean { .. } => {
                formatter.write_str("typed record batch archive boolean field must be 0 or 1")
            }
            Self::InvalidFieldTypeTag { .. } => {
                formatter.write_str("typed record batch archive field type tag is invalid")
            }
            Self::InvalidValueTag { .. } => {
                formatter.write_str("typed record batch archive value tag is invalid")
            }
            Self::TrailingBytes { .. } => {
                formatter.write_str("typed record batch archive contains trailing bytes")
            }
            Self::LengthOutOfRange { .. } => formatter
                .write_str("typed record batch archive length field is outside the runtime range"),
        }
    }
}

impl Error for FSETypedRecordBatchArchiveCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::UnexpectedEndOfArchive { .. }
            | Self::InvalidUtf8 { .. }
            | Self::InvalidBoolean { .. }
            | Self::InvalidFieldTypeTag { .. }
            | Self::InvalidValueTag { .. }
            | Self::TrailingBytes { .. }
            | Self::LengthOutOfRange { .. } => None,
        }
    }
}

impl From<FSETypedRecordBatchArchiveSnapshotError> for FSETypedRecordBatchArchiveCodecError {
    fn from(error: FSETypedRecordBatchArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

/// Encodes a typed record batch archive snapshot into little-endian bytes.
pub fn encode_typed_record_batch_archive_snapshot(
    snapshot: &FSERecordBatchArchiveSnapshot,
) -> Result<Vec<u8>, FSETypedRecordBatchArchiveCodecError> {
    snapshot.validate()?;

    let mut bytes = Vec::new();

    write_u64(&mut bytes, snapshot.schema_fields.len() as u64);
    for field in &snapshot.schema_fields {
        write_field(&mut bytes, field);
    }

    write_u64_vec(&mut bytes, &snapshot.row_ids);

    write_u64(&mut bytes, snapshot.records.len() as u64);
    for record in &snapshot.records {
        write_record(&mut bytes, record);
    }

    Ok(bytes)
}

/// Decodes a typed record batch archive snapshot from little-endian bytes.
pub fn decode_typed_record_batch_archive_snapshot(
    bytes: &[u8],
) -> Result<FSERecordBatchArchiveSnapshot, FSETypedRecordBatchArchiveCodecError> {
    let mut reader = TypedRecordBatchArchiveReader::new(bytes);
    let field_count = reader.read_len("typed_batch.schema.field_count")?;
    let mut schema_fields = Vec::with_capacity(field_count);

    for _ in 0..field_count {
        schema_fields.push(reader.read_field()?);
    }

    let row_ids = reader.read_u64_vec("typed_batch.row_ids")?;
    let record_count = reader.read_len("typed_batch.record_count")?;
    let mut records = Vec::with_capacity(record_count);

    for _ in 0..record_count {
        records.push(reader.read_record()?);
    }

    if reader.remaining() != 0 {
        return Err(FSETypedRecordBatchArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let snapshot = FSERecordBatchArchiveSnapshot {
        schema_fields,
        row_ids,
        records,
    };
    snapshot.validate()?;

    Ok(snapshot)
}

impl FSERecordBatchArchiveSnapshot {
    /// Encodes this typed record batch snapshot into little-endian archive bytes.
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, FSETypedRecordBatchArchiveCodecError> {
        encode_typed_record_batch_archive_snapshot(self)
    }

    /// Decodes a typed record batch snapshot from little-endian archive bytes.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, FSETypedRecordBatchArchiveCodecError> {
        decode_typed_record_batch_archive_snapshot(bytes)
    }
}

fn write_field(bytes: &mut Vec<u8>, field: &FSEFieldArchiveRecord) {
    write_string(bytes, &field.name);
    write_field_type_tag(bytes, field.field_type);
    write_bool(bytes, field.nullable);
}

fn write_record(bytes: &mut Vec<u8>, record: &FSERecordArchiveRecord) {
    write_u64(bytes, record.values.len() as u64);

    for value in &record.values {
        write_value(bytes, value);
    }
}

fn write_value(bytes: &mut Vec<u8>, value: &FSEValueArchiveRecord) {
    match value {
        FSEValueArchiveRecord::Null => {
            bytes.push(0);
        }
        FSEValueArchiveRecord::Integer(value) => {
            bytes.push(1);
            write_i64(bytes, *value);
        }
        FSEValueArchiveRecord::Float(value) => {
            bytes.push(2);
            write_f64(bytes, *value);
        }
        FSEValueArchiveRecord::Text(value) => {
            bytes.push(3);
            write_string(bytes, value);
        }
        FSEValueArchiveRecord::Boolean(value) => {
            bytes.push(4);
            write_bool(bytes, *value);
        }
        FSEValueArchiveRecord::TimestampMillis(value) => {
            bytes.push(5);
            write_i64(bytes, *value);
        }
        FSEValueArchiveRecord::Category(value) => {
            bytes.push(6);
            write_string(bytes, value);
        }
    }
}

fn write_field_type_tag(bytes: &mut Vec<u8>, tag: FSEFieldTypeArchiveTag) {
    bytes.push(match tag {
        FSEFieldTypeArchiveTag::Integer => 0,
        FSEFieldTypeArchiveTag::Float => 1,
        FSEFieldTypeArchiveTag::Text => 2,
        FSEFieldTypeArchiveTag::Boolean => 3,
        FSEFieldTypeArchiveTag::TimestampMillis => 4,
        FSEFieldTypeArchiveTag::Category => 5,
    });
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
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

struct TypedRecordBatchArchiveReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> TypedRecordBatchArchiveReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_field(
        &mut self,
    ) -> Result<FSEFieldArchiveRecord, FSETypedRecordBatchArchiveCodecError> {
        Ok(FSEFieldArchiveRecord {
            name: self.read_string("typed_batch.schema.field.name")?,
            field_type: self.read_field_type_tag("typed_batch.schema.field.field_type")?,
            nullable: self.read_bool("typed_batch.schema.field.nullable")?,
        })
    }

    fn read_record(
        &mut self,
    ) -> Result<FSERecordArchiveRecord, FSETypedRecordBatchArchiveCodecError> {
        let value_count = self.read_len("typed_batch.record.value_count")?;
        let mut values = Vec::with_capacity(value_count);

        for _ in 0..value_count {
            values.push(self.read_value("typed_batch.record.value")?);
        }

        Ok(FSERecordArchiveRecord { values })
    }

    fn read_value(
        &mut self,
        field: &'static str,
    ) -> Result<FSEValueArchiveRecord, FSETypedRecordBatchArchiveCodecError> {
        match self.read_u8(field)? {
            0 => Ok(FSEValueArchiveRecord::Null),
            1 => Ok(FSEValueArchiveRecord::Integer(self.read_i64(field)?)),
            2 => Ok(FSEValueArchiveRecord::Float(self.read_f64(field)?)),
            3 => Ok(FSEValueArchiveRecord::Text(self.read_string(field)?)),
            4 => Ok(FSEValueArchiveRecord::Boolean(self.read_bool(field)?)),
            5 => Ok(FSEValueArchiveRecord::TimestampMillis(
                self.read_i64(field)?,
            )),
            6 => Ok(FSEValueArchiveRecord::Category(self.read_string(field)?)),
            value => Err(FSETypedRecordBatchArchiveCodecError::InvalidValueTag { field, value }),
        }
    }

    fn read_field_type_tag(
        &mut self,
        field: &'static str,
    ) -> Result<FSEFieldTypeArchiveTag, FSETypedRecordBatchArchiveCodecError> {
        match self.read_u8(field)? {
            0 => Ok(FSEFieldTypeArchiveTag::Integer),
            1 => Ok(FSEFieldTypeArchiveTag::Float),
            2 => Ok(FSEFieldTypeArchiveTag::Text),
            3 => Ok(FSEFieldTypeArchiveTag::Boolean),
            4 => Ok(FSEFieldTypeArchiveTag::TimestampMillis),
            5 => Ok(FSEFieldTypeArchiveTag::Category),
            value => {
                Err(FSETypedRecordBatchArchiveCodecError::InvalidFieldTypeTag { field, value })
            }
        }
    }

    fn read_string(
        &mut self,
        field: &'static str,
    ) -> Result<String, FSETypedRecordBatchArchiveCodecError> {
        let length = self.read_len(field)?;
        let bytes = self.read_exact(field, length)?;

        String::from_utf8(bytes.to_vec()).map_err(|_| {
            FSETypedRecordBatchArchiveCodecError::InvalidUtf8 {
                field,
                bytes: bytes.to_vec(),
            }
        })
    }

    fn read_u64_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u64>, FSETypedRecordBatchArchiveCodecError> {
        let length = self.read_len(field)?;
        let byte_length = length.checked_mul(size_of::<u64>()).ok_or(
            FSETypedRecordBatchArchiveCodecError::LengthOutOfRange {
                field,
                length: length as u64,
            },
        )?;

        if byte_length > self.remaining() {
            return Err(
                FSETypedRecordBatchArchiveCodecError::UnexpectedEndOfArchive {
                    field,
                    needed: byte_length,
                    remaining: self.remaining(),
                },
            );
        }

        let mut values = Vec::with_capacity(length);
        for _ in 0..length {
            values.push(self.read_u64(field)?);
        }

        Ok(values)
    }

    fn read_len(
        &mut self,
        field: &'static str,
    ) -> Result<usize, FSETypedRecordBatchArchiveCodecError> {
        let length = self.read_u64(field)?;

        usize::try_from(length)
            .map_err(|_| FSETypedRecordBatchArchiveCodecError::LengthOutOfRange { field, length })
    }

    fn read_bool(
        &mut self,
        field: &'static str,
    ) -> Result<bool, FSETypedRecordBatchArchiveCodecError> {
        let value = self.read_u8(field)?;

        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(FSETypedRecordBatchArchiveCodecError::InvalidBoolean { field, value }),
        }
    }

    fn read_u8(&mut self, field: &'static str) -> Result<u8, FSETypedRecordBatchArchiveCodecError> {
        Ok(self.read_exact(field, 1)?[0])
    }

    fn read_i64(
        &mut self,
        field: &'static str,
    ) -> Result<i64, FSETypedRecordBatchArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<i64>())?;

        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_f64(
        &mut self,
        field: &'static str,
    ) -> Result<f64, FSETypedRecordBatchArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<f64>())?;

        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_u64(
        &mut self,
        field: &'static str,
    ) -> Result<u64, FSETypedRecordBatchArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<u64>())?;

        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<&'a [u8], FSETypedRecordBatchArchiveCodecError> {
        let remaining = self.remaining();
        let Some(end) = self.offset.checked_add(length) else {
            return Err(
                FSETypedRecordBatchArchiveCodecError::UnexpectedEndOfArchive {
                    field,
                    needed: length,
                    remaining,
                },
            );
        };

        if end > self.bytes.len() {
            return Err(
                FSETypedRecordBatchArchiveCodecError::UnexpectedEndOfArchive {
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
