//! Binary encoding for row-mapped archive snapshots.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use crate::persistence::{FSEArchiveCodecError, decode_archive_snapshot, encode_archive_snapshot};

use super::{
    FSELeafRowIdArchiveRecord, FSERowMappedArchiveSnapshotError, FSERowMappedIndexArchiveSnapshot,
};

/// Error returned when row-mapped archive byte encoding or decoding fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERowMappedArchiveCodecError {
    /// The row-mapped snapshot failed validation.
    Snapshot(FSERowMappedArchiveSnapshotError),

    /// The embedded index snapshot codec failed.
    IndexCodec(FSEArchiveCodecError),

    /// The byte slice ended before a complete row-mapping field could be read.
    UnexpectedEndOfArchive {
        /// Archive field being read.
        field: &'static str,
        /// Number of bytes required for the field.
        needed: usize,
        /// Number of bytes remaining in the input.
        remaining: usize,
    },

    /// The archive contained bytes after the decoded row-mapped snapshot.
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

    /// A compact integer vector mode was not recognized.
    InvalidCompactIntegerVectorMode {
        /// Archive field being read.
        field: &'static str,
        /// Mode byte found in the input.
        mode: u8,
    },
}

impl fmt::Display for FSERowMappedArchiveCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::IndexCodec(error) => error.fmt(formatter),
            Self::UnexpectedEndOfArchive { .. } => {
                formatter.write_str("row-mapped archive ended before the field could be read")
            }
            Self::TrailingBytes { .. } => {
                formatter.write_str("row-mapped archive contains trailing bytes")
            }
            Self::LengthOutOfRange { .. } => {
                formatter.write_str("row-mapped archive length field is outside the runtime range")
            }
            Self::InvalidCompactIntegerVectorMode { .. } => {
                formatter.write_str("row-mapped archive compact integer vector mode is invalid")
            }
        }
    }
}

impl Error for FSERowMappedArchiveCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::IndexCodec(error) => Some(error),
            Self::UnexpectedEndOfArchive { .. }
            | Self::TrailingBytes { .. }
            | Self::LengthOutOfRange { .. }
            | Self::InvalidCompactIntegerVectorMode { .. } => None,
        }
    }
}

impl From<FSERowMappedArchiveSnapshotError> for FSERowMappedArchiveCodecError {
    fn from(error: FSERowMappedArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<FSEArchiveCodecError> for FSERowMappedArchiveCodecError {
    fn from(error: FSEArchiveCodecError) -> Self {
        Self::IndexCodec(error)
    }
}

/// Encodes a row-mapped archive snapshot into little-endian bytes.
pub fn encode_row_mapped_archive_snapshot(
    snapshot: &FSERowMappedIndexArchiveSnapshot,
) -> Result<Vec<u8>, FSERowMappedArchiveCodecError> {
    snapshot.validate()?;

    let index_bytes = encode_archive_snapshot(&snapshot.index)
        .map_err(FSERowMappedArchiveCodecError::IndexCodec)?;
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&ROW_MAPPED_ARCHIVE_V2_MAGIC);
    write_byte_vec(&mut bytes, &index_bytes);
    write_u64(&mut bytes, snapshot.leaf_row_id_records.len() as u64);

    for record in &snapshot.leaf_row_id_records {
        write_u64(&mut bytes, record.node_id);
        write_compact_u64_vec(&mut bytes, &record.row_ids);
    }

    Ok(bytes)
}

/// Decodes a row-mapped archive snapshot from little-endian bytes.
pub fn decode_row_mapped_archive_snapshot(
    bytes: &[u8],
) -> Result<FSERowMappedIndexArchiveSnapshot, FSERowMappedArchiveCodecError> {
    if !bytes.starts_with(&ROW_MAPPED_ARCHIVE_V2_MAGIC) {
        return decode_legacy_row_mapped_archive_snapshot(bytes);
    }

    let mut reader = RowMappedArchiveReader::new(bytes);
    reader.read_exact("row_mapped.magic", ROW_MAPPED_ARCHIVE_V2_MAGIC.len())?;
    let index_bytes = reader.read_byte_vec("row_mapped.index_snapshot")?;
    let index =
        decode_archive_snapshot(&index_bytes).map_err(FSERowMappedArchiveCodecError::IndexCodec)?;
    let row_mapping_count = reader.read_len("row_mapped.leaf_row_id_record_count")?;
    let mut leaf_row_id_records = Vec::with_capacity(row_mapping_count);

    for _ in 0..row_mapping_count {
        leaf_row_id_records.push(FSELeafRowIdArchiveRecord {
            node_id: reader.read_u64("row_mapped.leaf_row_id_record.node_id")?,
            row_ids: reader.read_compact_u64_vec("row_mapped.leaf_row_id_record.row_ids")?,
        });
    }

    if reader.remaining() != 0 {
        return Err(FSERowMappedArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let snapshot = FSERowMappedIndexArchiveSnapshot {
        index,
        leaf_row_id_records,
    };
    snapshot.validate()?;

    Ok(snapshot)
}

fn decode_legacy_row_mapped_archive_snapshot(
    bytes: &[u8],
) -> Result<FSERowMappedIndexArchiveSnapshot, FSERowMappedArchiveCodecError> {
    let mut reader = RowMappedArchiveReader::new(bytes);
    let index_bytes = reader.read_byte_vec("row_mapped.index_snapshot")?;
    let index =
        decode_archive_snapshot(&index_bytes).map_err(FSERowMappedArchiveCodecError::IndexCodec)?;
    let row_mapping_count = reader.read_len("row_mapped.leaf_row_id_record_count")?;
    let mut leaf_row_id_records = Vec::with_capacity(row_mapping_count);

    for _ in 0..row_mapping_count {
        leaf_row_id_records.push(FSELeafRowIdArchiveRecord {
            node_id: reader.read_u64("row_mapped.leaf_row_id_record.node_id")?,
            row_ids: reader.read_u64_vec("row_mapped.leaf_row_id_record.row_ids")?,
        });
    }

    if reader.remaining() != 0 {
        return Err(FSERowMappedArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let snapshot = FSERowMappedIndexArchiveSnapshot {
        index,
        leaf_row_id_records,
    };
    snapshot.validate()?;

    Ok(snapshot)
}

impl FSERowMappedIndexArchiveSnapshot {
    /// Encodes this row-mapped snapshot into little-endian archive bytes.
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, FSERowMappedArchiveCodecError> {
        encode_row_mapped_archive_snapshot(self)
    }

    /// Decodes a row-mapped snapshot from little-endian archive bytes.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, FSERowMappedArchiveCodecError> {
        decode_row_mapped_archive_snapshot(bytes)
    }
}

fn write_byte_vec(bytes: &mut Vec<u8>, values: &[u8]) {
    write_u64(bytes, values.len() as u64);
    bytes.extend_from_slice(values);
}

fn write_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    write_u64(bytes, values.len() as u64);

    for value in values {
        write_u64(bytes, *value);
    }
}

fn write_compact_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    let raw_len = size_of::<u64>() * values.len();
    let varint_len = values
        .iter()
        .map(|value| var_u64_len(*value))
        .sum::<usize>();
    let delta_varint_len = delta_varint_u64_len(values);

    if delta_varint_len < raw_len && delta_varint_len < varint_len {
        bytes.push(COMPACT_U64_VEC_DELTA_VARINT_MODE);
        write_u64(bytes, values.len() as u64);
        write_delta_varint_u64_vec(bytes, values);
    } else if varint_len < raw_len {
        bytes.push(COMPACT_U64_VEC_VARINT_MODE);
        write_u64(bytes, values.len() as u64);

        for value in values {
            write_var_u64(bytes, *value);
        }
    } else {
        bytes.push(COMPACT_U64_VEC_RAW_MODE);
        write_u64_vec(bytes, values);
    }
}

fn write_delta_varint_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    let mut previous = 0_i64;

    for value in values {
        debug_assert!(*value <= i64::MAX as u64);
        let current = *value as i64;
        let delta = current - previous;
        write_var_u64(bytes, zig_zag_encode_i64(delta));
        previous = current;
    }
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_var_u64(bytes: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;

        if value != 0 {
            byte |= 0x80;
        }

        bytes.push(byte);

        if value == 0 {
            break;
        }
    }
}

fn var_u64_len(mut value: u64) -> usize {
    let mut len = 1;

    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }

    len
}

fn delta_varint_u64_len(values: &[u64]) -> usize {
    let mut previous = 0_i64;
    let mut len = 0;

    for value in values {
        if *value > i64::MAX as u64 {
            return usize::MAX;
        }

        let current = *value as i64;
        let Some(delta) = current.checked_sub(previous) else {
            return usize::MAX;
        };
        len += var_u64_len(zig_zag_encode_i64(delta));
        previous = current;
    }

    len
}

fn zig_zag_encode_i64(value: i64) -> u64 {
    ((value as u64) << 1) ^ ((value >> 63) as u64)
}

fn zig_zag_decode_i64(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

const ROW_MAPPED_ARCHIVE_V2_MAGIC: [u8; 8] = *b"FSERMV02";
const COMPACT_U64_VEC_RAW_MODE: u8 = 0;
const COMPACT_U64_VEC_VARINT_MODE: u8 = 1;
const COMPACT_U64_VEC_DELTA_VARINT_MODE: u8 = 2;

struct RowMappedArchiveReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> RowMappedArchiveReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_byte_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u8>, FSERowMappedArchiveCodecError> {
        let length = self.read_len(field)?;
        let bytes = self.read_exact(field, length)?;

        Ok(bytes.to_vec())
    }

    fn read_u64_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u64>, FSERowMappedArchiveCodecError> {
        let length = self.read_len(field)?;
        let byte_length = length.checked_mul(size_of::<u64>()).ok_or(
            FSERowMappedArchiveCodecError::LengthOutOfRange {
                field,
                length: length as u64,
            },
        )?;

        if byte_length > self.remaining() {
            return Err(FSERowMappedArchiveCodecError::UnexpectedEndOfArchive {
                field,
                needed: byte_length,
                remaining: self.remaining(),
            });
        }

        let mut values = Vec::with_capacity(length);
        for _ in 0..length {
            values.push(self.read_u64(field)?);
        }

        Ok(values)
    }

    fn read_compact_u64_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u64>, FSERowMappedArchiveCodecError> {
        match self.read_u8(field)? {
            COMPACT_U64_VEC_RAW_MODE => self.read_u64_vec(field),
            COMPACT_U64_VEC_VARINT_MODE => {
                let length = self.read_len(field)?;
                let mut values = Vec::with_capacity(length);

                for _ in 0..length {
                    values.push(self.read_var_u64(field)?);
                }

                Ok(values)
            }
            COMPACT_U64_VEC_DELTA_VARINT_MODE => {
                let length = self.read_len(field)?;
                self.read_delta_varint_u64_vec(field, length)
            }
            mode => {
                Err(FSERowMappedArchiveCodecError::InvalidCompactIntegerVectorMode { field, mode })
            }
        }
    }

    fn read_delta_varint_u64_vec(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<Vec<u64>, FSERowMappedArchiveCodecError> {
        let mut values = Vec::with_capacity(length);
        let mut previous = 0_i64;

        for _ in 0..length {
            let delta = zig_zag_decode_i64(self.read_var_u64(field)?);
            let Some(current) = previous.checked_add(delta) else {
                return Err(FSERowMappedArchiveCodecError::LengthOutOfRange {
                    field,
                    length: u64::MAX,
                });
            };

            if current < 0 {
                return Err(FSERowMappedArchiveCodecError::LengthOutOfRange {
                    field,
                    length: current as u64,
                });
            }

            values.push(current as u64);
            previous = current;
        }

        Ok(values)
    }

    fn read_len(&mut self, field: &'static str) -> Result<usize, FSERowMappedArchiveCodecError> {
        let length = self.read_u64(field)?;

        usize::try_from(length)
            .map_err(|_| FSERowMappedArchiveCodecError::LengthOutOfRange { field, length })
    }

    fn read_u64(&mut self, field: &'static str) -> Result<u64, FSERowMappedArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<u64>())?;

        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_u8(&mut self, field: &'static str) -> Result<u8, FSERowMappedArchiveCodecError> {
        let bytes = self.read_exact(field, 1)?;

        Ok(bytes[0])
    }

    fn read_var_u64(&mut self, field: &'static str) -> Result<u64, FSERowMappedArchiveCodecError> {
        let mut value = 0_u64;
        let mut shift = 0_u32;

        for _ in 0..10 {
            let byte = self.read_u8(field)?;
            value |= u64::from(byte & 0x7f) << shift;

            if byte & 0x80 == 0 {
                return Ok(value);
            }

            shift += 7;
        }

        Err(FSERowMappedArchiveCodecError::LengthOutOfRange {
            field,
            length: u64::MAX,
        })
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<&'a [u8], FSERowMappedArchiveCodecError> {
        let remaining = self.remaining();
        let Some(end) = self.offset.checked_add(length) else {
            return Err(FSERowMappedArchiveCodecError::UnexpectedEndOfArchive {
                field,
                needed: length,
                remaining,
            });
        };

        if end > self.bytes.len() {
            return Err(FSERowMappedArchiveCodecError::UnexpectedEndOfArchive {
                field,
                needed: length,
                remaining,
            });
        }

        let slice = &self.bytes[self.offset..end];
        self.offset = end;

        Ok(slice)
    }
}
