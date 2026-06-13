//! Binary encoding for typed row tombstone archive snapshots.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use super::{FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveSnapshotError};

/// Error returned when typed row tombstone archive byte encoding or decoding fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedRowTombstoneArchiveCodecError {
    /// The typed row tombstone snapshot failed validation.
    Snapshot(FSETypedRowTombstoneArchiveSnapshotError),

    /// The byte slice ended before a complete typed row tombstone field could be read.
    UnexpectedEndOfArchive {
        /// Archive field being read.
        field: &'static str,

        /// Number of bytes required for the field.
        needed: usize,

        /// Number of bytes remaining in the input.
        remaining: usize,
    },

    /// The archive contained bytes after the decoded typed row tombstone snapshot.
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

impl fmt::Display for FSETypedRowTombstoneArchiveCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::UnexpectedEndOfArchive { .. } => formatter
                .write_str("typed row tombstone archive ended before the field could be read"),
            Self::TrailingBytes { .. } => {
                formatter.write_str("typed row tombstone archive contains trailing bytes")
            }
            Self::LengthOutOfRange { .. } => formatter
                .write_str("typed row tombstone archive length field is outside the runtime range"),
        }
    }
}

impl Error for FSETypedRowTombstoneArchiveCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::UnexpectedEndOfArchive { .. }
            | Self::TrailingBytes { .. }
            | Self::LengthOutOfRange { .. } => None,
        }
    }
}

impl From<FSETypedRowTombstoneArchiveSnapshotError> for FSETypedRowTombstoneArchiveCodecError {
    fn from(error: FSETypedRowTombstoneArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

/// Encodes a typed row tombstone archive snapshot into little-endian bytes.
pub fn encode_typed_row_tombstone_archive_snapshot(
    snapshot: &FSETypedRowTombstoneArchiveSnapshot,
) -> Result<Vec<u8>, FSETypedRowTombstoneArchiveCodecError> {
    snapshot.validate()?;

    let mut bytes = Vec::new();
    write_u64_vec(&mut bytes, &snapshot.deleted_row_ids);

    Ok(bytes)
}

/// Decodes a typed row tombstone archive snapshot from little-endian bytes.
pub fn decode_typed_row_tombstone_archive_snapshot(
    bytes: &[u8],
) -> Result<FSETypedRowTombstoneArchiveSnapshot, FSETypedRowTombstoneArchiveCodecError> {
    let mut reader = TypedRowTombstoneArchiveReader::new(bytes);
    let deleted_row_ids = reader.read_u64_vec("typed_tombstone.deleted_row_ids")?;

    if reader.remaining() != 0 {
        return Err(FSETypedRowTombstoneArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let snapshot = FSETypedRowTombstoneArchiveSnapshot { deleted_row_ids };
    snapshot.validate()?;

    Ok(snapshot)
}

impl FSETypedRowTombstoneArchiveSnapshot {
    /// Encodes this typed row tombstone snapshot into little-endian archive bytes.
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, FSETypedRowTombstoneArchiveCodecError> {
        encode_typed_row_tombstone_archive_snapshot(self)
    }

    /// Decodes a typed row tombstone snapshot from little-endian archive bytes.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, FSETypedRowTombstoneArchiveCodecError> {
        decode_typed_row_tombstone_archive_snapshot(bytes)
    }
}

fn write_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    write_u64(bytes, values.len() as u64);

    for value in values {
        write_u64(bytes, *value);
    }
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

struct TypedRowTombstoneArchiveReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> TypedRowTombstoneArchiveReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_u64_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u64>, FSETypedRowTombstoneArchiveCodecError> {
        let length = self.read_len(field)?;
        let byte_length = length.checked_mul(size_of::<u64>()).ok_or(
            FSETypedRowTombstoneArchiveCodecError::LengthOutOfRange {
                field,
                length: length as u64,
            },
        )?;

        if byte_length > self.remaining() {
            return Err(
                FSETypedRowTombstoneArchiveCodecError::UnexpectedEndOfArchive {
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
    ) -> Result<usize, FSETypedRowTombstoneArchiveCodecError> {
        let length = self.read_u64(field)?;

        usize::try_from(length)
            .map_err(|_| FSETypedRowTombstoneArchiveCodecError::LengthOutOfRange { field, length })
    }

    fn read_u64(
        &mut self,
        field: &'static str,
    ) -> Result<u64, FSETypedRowTombstoneArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<u64>())?;

        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<&'a [u8], FSETypedRowTombstoneArchiveCodecError> {
        let remaining = self.remaining();
        let Some(end) = self.offset.checked_add(length) else {
            return Err(
                FSETypedRowTombstoneArchiveCodecError::UnexpectedEndOfArchive {
                    field,
                    needed: length,
                    remaining,
                },
            );
        };

        if end > self.bytes.len() {
            return Err(
                FSETypedRowTombstoneArchiveCodecError::UnexpectedEndOfArchive {
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
