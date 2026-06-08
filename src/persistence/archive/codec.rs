//! Binary archive snapshot encoding.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use crate::math::Scalar;

use super::{FSEArchiveSnapshotError, FSEIndexArchiveSnapshot, FSEPartitionNodeArchiveRecord};
use crate::persistence::{FSEArchiveManifest, FSEArchiveSections};

const MIN_ARCHIVE_NODE_BYTES: usize = 57;

/// Error returned when archive byte encoding or decoding fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveCodecError {
    /// The snapshot failed archive validation.
    Snapshot(FSEArchiveSnapshotError),

    /// The byte slice ended before a complete archive field could be read.
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

    /// The archive contained bytes after the decoded snapshot.
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

impl fmt::Display for FSEArchiveCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => error.fmt(formatter),
            Self::UnexpectedEndOfArchive { .. } => {
                formatter.write_str("archive ended before the field could be read")
            }
            Self::InvalidUtf8 { .. } => {
                formatter.write_str("archive string field is not valid UTF-8")
            }
            Self::InvalidBoolean { .. } => {
                formatter.write_str("archive boolean field must be 0 or 1")
            }
            Self::TrailingBytes { .. } => formatter.write_str("archive contains trailing bytes"),
            Self::LengthOutOfRange { .. } => {
                formatter.write_str("archive length field is outside the runtime range")
            }
        }
    }
}

impl Error for FSEArchiveCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::UnexpectedEndOfArchive { .. }
            | Self::InvalidUtf8 { .. }
            | Self::InvalidBoolean { .. }
            | Self::TrailingBytes { .. }
            | Self::LengthOutOfRange { .. } => None,
        }
    }
}

impl From<FSEArchiveSnapshotError> for FSEArchiveCodecError {
    fn from(error: FSEArchiveSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

/// Encodes an archive snapshot into little-endian bytes.
pub fn encode_archive_snapshot(
    snapshot: &FSEIndexArchiveSnapshot,
) -> Result<Vec<u8>, FSEArchiveCodecError> {
    snapshot.validate()?;

    let mut bytes = Vec::new();
    write_manifest(&mut bytes, &snapshot.manifest);

    for node in &snapshot.nodes {
        write_node(&mut bytes, node);
    }

    Ok(bytes)
}

/// Decodes an archive snapshot from little-endian bytes.
pub fn decode_archive_snapshot(
    bytes: &[u8],
) -> Result<FSEIndexArchiveSnapshot, FSEArchiveCodecError> {
    let mut reader = ArchiveReader::new(bytes);
    let manifest = reader.read_manifest()?;

    manifest
        .validate()
        .map_err(FSEArchiveSnapshotError::Manifest)?;

    let node_count = reader.read_node_count(manifest.node_count)?;
    let mut nodes = Vec::with_capacity(node_count);

    for _ in 0..node_count {
        nodes.push(reader.read_node()?);
    }

    if reader.remaining() != 0 {
        return Err(FSEArchiveCodecError::TrailingBytes {
            remaining: reader.remaining(),
        });
    }

    let snapshot = FSEIndexArchiveSnapshot { manifest, nodes };
    snapshot.validate()?;

    Ok(snapshot)
}

impl FSEIndexArchiveSnapshot {
    /// Encodes this snapshot into little-endian archive bytes.
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, FSEArchiveCodecError> {
        encode_archive_snapshot(self)
    }

    /// Decodes a snapshot from little-endian archive bytes.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, FSEArchiveCodecError> {
        decode_archive_snapshot(bytes)
    }
}

fn write_manifest(bytes: &mut Vec<u8>, manifest: &FSEArchiveManifest) {
    write_string(bytes, &manifest.magic);
    write_u32(bytes, manifest.format_version);
    write_string(bytes, &manifest.file_extension);
    write_u32(bytes, manifest.scalar_size_bytes);
    write_u32(bytes, manifest.dimensions);
    write_u64(bytes, manifest.record_count);
    write_u64(bytes, manifest.node_count);
    write_u64(bytes, manifest.root_node_id);
    write_bool(bytes, manifest.sections.dataset_metadata);
    write_bool(bytes, manifest.sections.schema_metadata);
    write_bool(bytes, manifest.sections.encoder_metadata);
}

fn write_node(bytes: &mut Vec<u8>, node: &FSEPartitionNodeArchiveRecord) {
    write_u64(bytes, node.id);
    write_scalar_vec(bytes, &node.centroid);
    write_scalar_vec(bytes, &node.bounds_min);
    write_scalar_vec(bytes, &node.bounds_max);
    write_u64(bytes, node.residual_dimensions.len() as u64);

    for dimension in &node.residual_dimensions {
        write_scalar_vec(bytes, dimension);
    }

    write_u64(bytes, node.cardinality);
    write_u64_vec(bytes, &node.children);
    write_bool(bytes, node.is_leaf);
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn write_scalar_vec(bytes: &mut Vec<u8>, values: &[Scalar]) {
    write_u64(bytes, values.len() as u64);

    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
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

fn write_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

struct ArchiveReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ArchiveReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_manifest(&mut self) -> Result<FSEArchiveManifest, FSEArchiveCodecError> {
        Ok(FSEArchiveManifest {
            magic: self.read_string("manifest.magic")?,
            format_version: self.read_u32("manifest.format_version")?,
            file_extension: self.read_string("manifest.file_extension")?,
            scalar_size_bytes: self.read_u32("manifest.scalar_size_bytes")?,
            dimensions: self.read_u32("manifest.dimensions")?,
            record_count: self.read_u64("manifest.record_count")?,
            node_count: self.read_u64("manifest.node_count")?,
            root_node_id: self.read_u64("manifest.root_node_id")?,
            sections: FSEArchiveSections {
                dataset_metadata: self.read_bool("manifest.sections.dataset_metadata")?,
                schema_metadata: self.read_bool("manifest.sections.schema_metadata")?,
                encoder_metadata: self.read_bool("manifest.sections.encoder_metadata")?,
            },
        })
    }

    fn read_node_count(&self, value: u64) -> Result<usize, FSEArchiveCodecError> {
        let node_count = checked_len("manifest.node_count", value)?;
        let minimum_bytes = node_count.checked_mul(MIN_ARCHIVE_NODE_BYTES).ok_or(
            FSEArchiveCodecError::LengthOutOfRange {
                field: "manifest.node_count",
                length: value,
            },
        )?;

        if minimum_bytes > self.remaining() {
            return Err(FSEArchiveCodecError::UnexpectedEndOfArchive {
                field: "nodes",
                needed: minimum_bytes,
                remaining: self.remaining(),
            });
        }

        Ok(node_count)
    }

    fn read_node(&mut self) -> Result<FSEPartitionNodeArchiveRecord, FSEArchiveCodecError> {
        let id = self.read_u64("node.id")?;
        let centroid = self.read_scalar_vec("node.centroid")?;
        let bounds_min = self.read_scalar_vec("node.bounds_min")?;
        let bounds_max = self.read_scalar_vec("node.bounds_max")?;
        let residual_dimension_count = checked_len(
            "node.residual_dimension_count",
            self.read_u64("node.residual_dimension_count")?,
        )?;
        let mut residual_dimensions = Vec::with_capacity(residual_dimension_count);

        for _ in 0..residual_dimension_count {
            residual_dimensions.push(self.read_scalar_vec("node.residual_dimension")?);
        }

        Ok(FSEPartitionNodeArchiveRecord {
            id,
            centroid,
            bounds_min,
            bounds_max,
            residual_dimensions,
            cardinality: self.read_u64("node.cardinality")?,
            children: self.read_u64_vec("node.children")?,
            is_leaf: self.read_bool("node.is_leaf")?,
        })
    }

    fn read_string(&mut self, field: &'static str) -> Result<String, FSEArchiveCodecError> {
        let length = checked_len(field, self.read_u64(field)?)?;
        let bytes = self.read_exact(field, length)?;

        String::from_utf8(bytes.to_vec()).map_err(|_| FSEArchiveCodecError::InvalidUtf8 {
            field,
            bytes: bytes.to_vec(),
        })
    }

    fn read_scalar_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<Scalar>, FSEArchiveCodecError> {
        let length = checked_len(field, self.read_u64(field)?)?;
        let byte_length = length.checked_mul(size_of::<Scalar>()).ok_or(
            FSEArchiveCodecError::LengthOutOfRange {
                field,
                length: length as u64,
            },
        )?;
        let bytes = self.read_exact(field, byte_length)?;
        let mut values = Vec::with_capacity(length);

        for chunk in bytes.chunks_exact(size_of::<Scalar>()) {
            values.push(Scalar::from_le_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3],
            ]));
        }

        Ok(values)
    }

    fn read_u64_vec(&mut self, field: &'static str) -> Result<Vec<u64>, FSEArchiveCodecError> {
        let length = checked_len(field, self.read_u64(field)?)?;
        let byte_length =
            length
                .checked_mul(size_of::<u64>())
                .ok_or(FSEArchiveCodecError::LengthOutOfRange {
                    field,
                    length: length as u64,
                })?;

        if byte_length > self.remaining() {
            return Err(FSEArchiveCodecError::UnexpectedEndOfArchive {
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

    fn read_bool(&mut self, field: &'static str) -> Result<bool, FSEArchiveCodecError> {
        let value = self.read_exact(field, 1)?[0];

        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(FSEArchiveCodecError::InvalidBoolean { field, value }),
        }
    }

    fn read_u32(&mut self, field: &'static str) -> Result<u32, FSEArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<u32>())?;

        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self, field: &'static str) -> Result<u64, FSEArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<u64>())?;

        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<&'a [u8], FSEArchiveCodecError> {
        let remaining = self.remaining();
        let Some(end) = self.offset.checked_add(length) else {
            return Err(FSEArchiveCodecError::UnexpectedEndOfArchive {
                field,
                needed: length,
                remaining,
            });
        };

        if end > self.bytes.len() {
            return Err(FSEArchiveCodecError::UnexpectedEndOfArchive {
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

fn checked_len(field: &'static str, length: u64) -> Result<usize, FSEArchiveCodecError> {
    usize::try_from(length).map_err(|_| FSEArchiveCodecError::LengthOutOfRange { field, length })
}
