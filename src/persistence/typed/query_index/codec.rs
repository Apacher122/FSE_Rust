//! Binary encoding for typed query index archive snapshots.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

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
}

impl fmt::Display for FSETypedQueryIndexArchiveCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::IndexCodec(error) => error.fmt(formatter),
            Self::BatchCodec(error) => error.fmt(formatter),
            Self::UnexpectedEndOfArchive { .. } => formatter
                .write_str("typed query index archive ended before the field could be read"),
            Self::TrailingBytes { .. } => {
                formatter.write_str("typed query index archive contains trailing bytes")
            }
            Self::LengthOutOfRange { .. } => formatter
                .write_str("typed query index archive length field is outside the runtime range"),
        }
    }
}

impl Error for FSETypedQueryIndexArchiveCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::IndexCodec(error) => Some(error),
            Self::BatchCodec(error) => Some(error),
            Self::UnexpectedEndOfArchive { .. }
            | Self::TrailingBytes { .. }
            | Self::LengthOutOfRange { .. } => None,
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

/// Encodes a typed query index archive snapshot into little-endian bytes.
pub fn encode_typed_query_index_archive_snapshot(
    snapshot: &FSETypedQueryIndexArchiveSnapshot,
) -> Result<Vec<u8>, FSETypedQueryIndexArchiveCodecError> {
    snapshot.validate()?;

    let index_bytes = encode_row_mapped_archive_snapshot(&snapshot.index)
        .map_err(FSETypedQueryIndexArchiveCodecError::IndexCodec)?;
    let batch_bytes = encode_typed_record_batch_archive_snapshot(&snapshot.batch)
        .map_err(FSETypedQueryIndexArchiveCodecError::BatchCodec)?;
    let mut bytes = Vec::new();

    write_byte_vec(&mut bytes, &index_bytes);
    write_byte_vec(&mut bytes, &batch_bytes);

    Ok(bytes)
}

/// Decodes a typed query index archive snapshot from little-endian bytes.
pub fn decode_typed_query_index_archive_snapshot(
    bytes: &[u8],
) -> Result<FSETypedQueryIndexArchiveSnapshot, FSETypedQueryIndexArchiveCodecError> {
    let mut reader = TypedQueryIndexArchiveReader::new(bytes);
    let index_bytes = reader.read_byte_vec("typed_index.row_mapped_index")?;
    let batch_bytes = reader.read_byte_vec("typed_index.record_batch")?;

    if reader.remaining() != 0 {
        return Err(FSETypedQueryIndexArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let index = decode_row_mapped_archive_snapshot(&index_bytes)
        .map_err(FSETypedQueryIndexArchiveCodecError::IndexCodec)?;
    let batch = decode_typed_record_batch_archive_snapshot(&batch_bytes)
        .map_err(FSETypedQueryIndexArchiveCodecError::BatchCodec)?;
    let snapshot = FSETypedQueryIndexArchiveSnapshot { index, batch };
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

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

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
