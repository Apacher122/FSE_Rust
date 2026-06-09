//! Payload metadata for `.fse` archive files.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

/// Fixed byte marker written before every `.fse` archive file payload.
pub const FSE_ARCHIVE_PAYLOAD_MAGIC: [u8; 8] = *b"FSEPLD01";

/// Current `.fse` archive payload header version.
pub const FSE_ARCHIVE_PAYLOAD_HEADER_VERSION: u32 = 1;

/// Logical payload stored in an `.fse` archive file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEArchivePayloadKind {
    /// Numeric FSE index archive.
    Index,

    /// Row-mapped FSE index archive.
    RowMappedIndex,

    /// Typed record batch archive.
    TypedRecordBatch,

    /// Typed query index archive.
    TypedQueryIndex,
}

impl FSEArchivePayloadKind {
    fn tag(self) -> u8 {
        match self {
            Self::Index => 1,
            Self::RowMappedIndex => 2,
            Self::TypedRecordBatch => 3,
            Self::TypedQueryIndex => 4,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, FSEArchivePayloadHeaderError> {
        match tag {
            1 => Ok(Self::Index),
            2 => Ok(Self::RowMappedIndex),
            3 => Ok(Self::TypedRecordBatch),
            4 => Ok(Self::TypedQueryIndex),
            value => Err(FSEArchivePayloadHeaderError::UnknownPayloadKind { value }),
        }
    }
}

/// Error returned when an `.fse` archive payload header is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchivePayloadHeaderError {
    /// The byte slice ended before a complete payload header field could be read.
    UnexpectedEndOfArchive {
        /// Header field being read.
        field: &'static str,
        /// Number of bytes required for the field.
        needed: usize,
        /// Number of bytes remaining in the input.
        remaining: usize,
    },

    /// The payload header marker did not match the FSE archive marker.
    InvalidPayloadMagic {
        /// Marker found in the input.
        actual: [u8; 8],
    },

    /// The payload header version is not supported by this runtime.
    UnsupportedPayloadHeaderVersion {
        /// Version found in the input.
        actual: u32,
        /// Version supported by this runtime.
        expected: u32,
    },

    /// The payload kind tag did not match a supported archive payload kind.
    UnknownPayloadKind {
        /// Raw payload kind tag found in the input.
        value: u8,
    },

    /// The payload kind does not match the archive reader.
    UnexpectedPayloadKind {
        /// Payload kind required by the archive reader.
        expected: FSEArchivePayloadKind,
        /// Payload kind found in the input.
        actual: FSEArchivePayloadKind,
    },
}

impl fmt::Display for FSEArchivePayloadHeaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEndOfArchive { .. } => {
                formatter.write_str("archive ended before the payload header field could be read")
            }
            Self::InvalidPayloadMagic { .. } => {
                formatter.write_str("archive payload header marker is invalid")
            }
            Self::UnsupportedPayloadHeaderVersion { .. } => {
                formatter.write_str("archive payload header version is not supported")
            }
            Self::UnknownPayloadKind { .. } => {
                formatter.write_str("archive payload kind tag is invalid")
            }
            Self::UnexpectedPayloadKind { .. } => {
                formatter.write_str("archive payload kind does not match the reader")
            }
        }
    }
}

impl Error for FSEArchivePayloadHeaderError {}

/// Encodes an archive payload with file-level payload metadata.
pub fn encode_archive_payload(kind: FSEArchivePayloadKind, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        FSE_ARCHIVE_PAYLOAD_MAGIC.len() + size_of::<u32>() + size_of::<u8>() + payload.len(),
    );

    bytes.extend_from_slice(&FSE_ARCHIVE_PAYLOAD_MAGIC);
    bytes.extend_from_slice(&FSE_ARCHIVE_PAYLOAD_HEADER_VERSION.to_le_bytes());
    bytes.push(kind.tag());
    bytes.extend_from_slice(payload);

    bytes
}

/// Decodes an archive payload and verifies its file-level payload kind.
pub fn decode_archive_payload(
    expected_kind: FSEArchivePayloadKind,
    bytes: &[u8],
) -> Result<Vec<u8>, FSEArchivePayloadHeaderError> {
    let mut reader = ArchivePayloadHeaderReader::new(bytes);
    let magic = reader.read_magic("payload.magic")?;

    if magic != FSE_ARCHIVE_PAYLOAD_MAGIC {
        return Err(FSEArchivePayloadHeaderError::InvalidPayloadMagic { actual: magic });
    }

    let version = reader.read_u32("payload.header_version")?;
    if version != FSE_ARCHIVE_PAYLOAD_HEADER_VERSION {
        return Err(
            FSEArchivePayloadHeaderError::UnsupportedPayloadHeaderVersion {
                actual: version,
                expected: FSE_ARCHIVE_PAYLOAD_HEADER_VERSION,
            },
        );
    }

    let actual_kind = FSEArchivePayloadKind::from_tag(reader.read_u8("payload.kind")?)?;
    if actual_kind != expected_kind {
        return Err(FSEArchivePayloadHeaderError::UnexpectedPayloadKind {
            expected: expected_kind,
            actual: actual_kind,
        });
    }

    Ok(reader.remaining_bytes().to_vec())
}

struct ArchivePayloadHeaderReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ArchivePayloadHeaderReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn remaining_bytes(&self) -> &'a [u8] {
        &self.bytes[self.offset..]
    }

    fn read_magic(&mut self, field: &'static str) -> Result<[u8; 8], FSEArchivePayloadHeaderError> {
        let bytes = self.read_exact(field, FSE_ARCHIVE_PAYLOAD_MAGIC.len())?;

        Ok([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }

    fn read_u32(&mut self, field: &'static str) -> Result<u32, FSEArchivePayloadHeaderError> {
        let bytes = self.read_exact(field, size_of::<u32>())?;

        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u8(&mut self, field: &'static str) -> Result<u8, FSEArchivePayloadHeaderError> {
        Ok(self.read_exact(field, size_of::<u8>())?[0])
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<&'a [u8], FSEArchivePayloadHeaderError> {
        let remaining = self.remaining();
        let Some(end) = self.offset.checked_add(length) else {
            return Err(FSEArchivePayloadHeaderError::UnexpectedEndOfArchive {
                field,
                needed: length,
                remaining,
            });
        };

        if end > self.bytes.len() {
            return Err(FSEArchivePayloadHeaderError::UnexpectedEndOfArchive {
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
