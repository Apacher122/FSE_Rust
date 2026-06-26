//! Binary archive snapshot encoding.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use crate::math::Scalar;

use super::{FSEArchiveSnapshotError, FSEIndexArchiveSnapshot, FSEPartitionNodeArchiveRecord};
use crate::persistence::{FSEArchiveManifest, FSEArchiveSections};

const MIN_ARCHIVE_NODE_BYTES: usize = 57;
const INDEX_ARCHIVE_V2_MAGIC: [u8; 8] = *b"FSEIDX02";
const INDEX_ARCHIVE_V3_MAGIC: [u8; 8] = *b"FSEIDX03";
const INDEX_ARCHIVE_V4_MAGIC: [u8; 8] = *b"FSEIDX04";
const INDEX_ARCHIVE_V5_MAGIC: [u8; 8] = *b"FSEIDX05";
const COMPACT_SCALAR_VEC_RAW_MODE: u8 = 0;
const COMPACT_SCALAR_VEC_EMPTY_MODE: u8 = 1;
const COMPACT_SCALAR_VEC_REPEATED_MODE: u8 = 2;
const COMPACT_SCALAR_VEC_BYTE_PLANES_MODE: u8 = 3;
const COMPACT_SCALAR_VEC_INTEGER_VARINT_MODE: u8 = 4;
const COMPACT_SCALAR_BYTE_PLANE_REPEATED_MODE: u8 = 0;
const COMPACT_SCALAR_BYTE_PLANE_RAW_MODE: u8 = 1;
const COMPACT_U64_VEC_RAW_MODE: u8 = 0;
const COMPACT_U64_VEC_VARINT_MODE: u8 = 1;
const COMPACT_U64_VEC_EMPTY_MODE: u8 = 2;

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

    /// A compact scalar vector mode was not recognized.
    InvalidCompactScalarVectorMode {
        /// Archive field being read.
        field: &'static str,
        /// Mode byte found in the input.
        mode: u8,
    },

    /// A compact scalar byte-plane mode was not recognized.
    InvalidCompactScalarBytePlaneMode {
        /// Archive field being read.
        field: &'static str,
        /// Mode byte found in the input.
        mode: u8,
    },

    /// A compact integer vector mode was not recognized.
    InvalidCompactIntegerVectorMode {
        /// Archive field being read.
        field: &'static str,
        /// Mode byte found in the input.
        mode: u8,
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
            Self::InvalidCompactScalarVectorMode { .. } => {
                formatter.write_str("archive compact scalar vector mode is invalid")
            }
            Self::InvalidCompactScalarBytePlaneMode { .. } => {
                formatter.write_str("archive compact scalar byte-plane mode is invalid")
            }
            Self::InvalidCompactIntegerVectorMode { .. } => {
                formatter.write_str("archive compact integer vector mode is invalid")
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
            | Self::LengthOutOfRange { .. }
            | Self::InvalidCompactScalarVectorMode { .. }
            | Self::InvalidCompactScalarBytePlaneMode { .. }
            | Self::InvalidCompactIntegerVectorMode { .. } => None,
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
    bytes.extend_from_slice(&INDEX_ARCHIVE_V5_MAGIC);
    write_manifest(&mut bytes, &snapshot.manifest);

    for node in &snapshot.nodes {
        write_fixed_shape_node(&mut bytes, node, snapshot.manifest.dimensions as usize);
    }

    Ok(bytes)
}

/// Decodes an archive snapshot from little-endian bytes.
pub fn decode_archive_snapshot(
    bytes: &[u8],
) -> Result<FSEIndexArchiveSnapshot, FSEArchiveCodecError> {
    if bytes.starts_with(&INDEX_ARCHIVE_V5_MAGIC) {
        return decode_fixed_shape_archive_snapshot(bytes, &INDEX_ARCHIVE_V5_MAGIC);
    }

    if bytes.starts_with(&INDEX_ARCHIVE_V4_MAGIC) {
        return decode_fixed_shape_archive_snapshot(bytes, &INDEX_ARCHIVE_V4_MAGIC);
    }

    if bytes.starts_with(&INDEX_ARCHIVE_V3_MAGIC) {
        return decode_fixed_shape_archive_snapshot(bytes, &INDEX_ARCHIVE_V3_MAGIC);
    }

    if !bytes.starts_with(&INDEX_ARCHIVE_V2_MAGIC) {
        return decode_legacy_archive_snapshot(bytes);
    }

    let mut reader = ArchiveReader::new(bytes);
    reader.read_exact("archive.magic", INDEX_ARCHIVE_V2_MAGIC.len())?;
    let manifest = reader.read_manifest()?;

    manifest
        .validate()
        .map_err(FSEArchiveSnapshotError::Manifest)?;

    let node_count = reader.read_node_count(manifest.node_count)?;
    let mut nodes = Vec::with_capacity(node_count);

    for _ in 0..node_count {
        nodes.push(reader.read_compact_node()?);
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

fn decode_fixed_shape_archive_snapshot(
    bytes: &[u8],
    magic: &[u8; 8],
) -> Result<FSEIndexArchiveSnapshot, FSEArchiveCodecError> {
    let mut reader = ArchiveReader::new(bytes);
    reader.read_exact("archive.magic", magic.len())?;
    let manifest = reader.read_manifest()?;

    manifest
        .validate()
        .map_err(FSEArchiveSnapshotError::Manifest)?;

    let dimensions = checked_len("manifest.dimensions", u64::from(manifest.dimensions))?;
    let node_count = reader.read_node_count(manifest.node_count)?;
    let mut nodes = Vec::with_capacity(node_count);

    for _ in 0..node_count {
        nodes.push(reader.read_fixed_shape_node(dimensions)?);
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

fn decode_legacy_archive_snapshot(
    bytes: &[u8],
) -> Result<FSEIndexArchiveSnapshot, FSEArchiveCodecError> {
    let mut reader = ArchiveReader::new(bytes);
    let manifest = reader.read_manifest()?;

    manifest
        .validate()
        .map_err(FSEArchiveSnapshotError::Manifest)?;

    let node_count = reader.read_legacy_node_count(manifest.node_count)?;
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

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn write_fixed_shape_node(
    bytes: &mut Vec<u8>,
    node: &FSEPartitionNodeArchiveRecord,
    dimensions: usize,
) {
    write_var_u64(bytes, node.id);
    write_fixed_compact_scalar_vec(bytes, &node.centroid, dimensions);
    write_fixed_compact_scalar_vec(bytes, &node.bounds_min, dimensions);
    write_fixed_compact_scalar_vec(bytes, &node.bounds_max, dimensions);

    let residual_rows = node.residual_dimensions.first().map_or(0, Vec::len);
    write_var_u64(bytes, residual_rows as u64);

    if residual_rows > 0 {
        for dimension in &node.residual_dimensions {
            write_fixed_compact_scalar_vec(bytes, dimension, residual_rows);
        }
    }

    write_var_u64(bytes, node.cardinality);
    write_compact_u64_vec(bytes, &node.children);
    write_bool(bytes, node.is_leaf);
}

fn write_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    write_u64(bytes, values.len() as u64);

    for value in values {
        write_u64(bytes, *value);
    }
}

fn write_fixed_compact_scalar_vec(bytes: &mut Vec<u8>, values: &[Scalar], expected_len: usize) {
    if expected_len == 0 {
        bytes.push(COMPACT_SCALAR_VEC_EMPTY_MODE);
        return;
    }

    if let Some(value) = repeated_scalar(values) {
        bytes.push(COMPACT_SCALAR_VEC_REPEATED_MODE);
        write_scalar(bytes, value);
        return;
    }

    let raw_len = size_of::<Scalar>() * values.len();
    let byte_plane_len = compact_scalar_byte_plane_payload_len(values);

    if let Some(integer_varint_len) = compact_scalar_integer_varint_payload_len(values) {
        if integer_varint_len < raw_len && integer_varint_len <= byte_plane_len {
            bytes.push(COMPACT_SCALAR_VEC_INTEGER_VARINT_MODE);
            write_scalar_integer_varints(bytes, values);
            return;
        }
    }

    if byte_plane_len < raw_len {
        bytes.push(COMPACT_SCALAR_VEC_BYTE_PLANES_MODE);
        write_scalar_byte_planes(bytes, values);
        return;
    }

    bytes.push(COMPACT_SCALAR_VEC_RAW_MODE);
    for value in values {
        write_scalar(bytes, *value);
    }
}

fn write_compact_u64_vec(bytes: &mut Vec<u8>, values: &[u64]) {
    if values.is_empty() {
        bytes.push(COMPACT_U64_VEC_EMPTY_MODE);
        return;
    }

    let raw_len = size_of::<u64>() * values.len();
    let varint_len = values
        .iter()
        .map(|value| var_u64_len(*value))
        .sum::<usize>();

    if varint_len < raw_len {
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

fn repeated_scalar(values: &[Scalar]) -> Option<Scalar> {
    if values.len() < 2 {
        return None;
    }

    let value = values[0];

    values
        .iter()
        .all(|other| other.to_bits() == value.to_bits())
        .then_some(value)
}

fn compact_scalar_byte_plane_payload_len(values: &[Scalar]) -> usize {
    (0..size_of::<Scalar>())
        .map(|lane| {
            if repeated_scalar_byte_lane(values, lane).is_some() {
                2
            } else {
                1 + values.len()
            }
        })
        .sum()
}

fn compact_scalar_integer_varint_payload_len(values: &[Scalar]) -> Option<usize> {
    let mut len = 0;

    for value in values {
        let integer = scalar_to_compact_integer(*value)?;
        len += var_u64_len(zigzag_i64(integer));
    }

    Some(len)
}

fn scalar_to_compact_integer(value: Scalar) -> Option<i64> {
    if !value.is_finite() || value.to_bits() == (-0.0_f32).to_bits() || value.fract() != 0.0 {
        return None;
    }

    if value < i64::MIN as Scalar || value > i64::MAX as Scalar {
        return None;
    }

    let integer = value as i64;

    ((integer as Scalar).to_bits() == value.to_bits()).then_some(integer)
}

fn repeated_scalar_byte_lane(values: &[Scalar], lane: usize) -> Option<u8> {
    let value = values.first()?.to_le_bytes()[lane];

    values
        .iter()
        .all(|other| other.to_le_bytes()[lane] == value)
        .then_some(value)
}

fn write_scalar_integer_varints(bytes: &mut Vec<u8>, values: &[Scalar]) {
    for value in values {
        if let Some(integer) = scalar_to_compact_integer(*value) {
            write_var_u64(bytes, zigzag_i64(integer));
        }
    }
}

fn write_scalar_byte_planes(bytes: &mut Vec<u8>, values: &[Scalar]) {
    for lane in 0..size_of::<Scalar>() {
        if let Some(value) = repeated_scalar_byte_lane(values, lane) {
            bytes.push(COMPACT_SCALAR_BYTE_PLANE_REPEATED_MODE);
            bytes.push(value);
        } else {
            bytes.push(COMPACT_SCALAR_BYTE_PLANE_RAW_MODE);

            for value in values {
                bytes.push(value.to_le_bytes()[lane]);
            }
        }
    }
}

fn zigzag_i64(value: i64) -> u64 {
    ((value as u64) << 1) ^ ((value >> 63) as u64)
}

fn unzigzag_i64(value: u64) -> i64 {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
}

fn write_scalar(bytes: &mut Vec<u8>, value: Scalar) {
    bytes.extend_from_slice(&value.to_le_bytes());
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
        checked_len("manifest.node_count", value)
    }

    fn read_legacy_node_count(&self, value: u64) -> Result<usize, FSEArchiveCodecError> {
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

    fn read_compact_node(&mut self) -> Result<FSEPartitionNodeArchiveRecord, FSEArchiveCodecError> {
        let id = self.read_u64("node.id")?;
        let centroid = self.read_compact_scalar_vec("node.centroid")?;
        let bounds_min = self.read_compact_scalar_vec("node.bounds_min")?;
        let bounds_max = self.read_compact_scalar_vec("node.bounds_max")?;
        let residual_dimension_count = checked_len(
            "node.residual_dimension_count",
            self.read_u64("node.residual_dimension_count")?,
        )?;
        let mut residual_dimensions = Vec::with_capacity(residual_dimension_count);

        for _ in 0..residual_dimension_count {
            residual_dimensions.push(self.read_compact_scalar_vec("node.residual_dimension")?);
        }

        Ok(FSEPartitionNodeArchiveRecord {
            id,
            centroid,
            bounds_min,
            bounds_max,
            residual_dimensions,
            cardinality: self.read_u64("node.cardinality")?,
            children: self.read_compact_u64_vec("node.children")?,
            is_leaf: self.read_bool("node.is_leaf")?,
        })
    }

    fn read_fixed_shape_node(
        &mut self,
        dimensions: usize,
    ) -> Result<FSEPartitionNodeArchiveRecord, FSEArchiveCodecError> {
        let id = self.read_var_u64("node.id")?;
        let centroid = self.read_fixed_compact_scalar_vec("node.centroid", dimensions)?;
        let bounds_min = self.read_fixed_compact_scalar_vec("node.bounds_min", dimensions)?;
        let bounds_max = self.read_fixed_compact_scalar_vec("node.bounds_max", dimensions)?;
        let residual_rows = checked_len(
            "node.residual_row_count",
            self.read_var_u64("node.residual_row_count")?,
        )?;
        let mut residual_dimensions = Vec::with_capacity(dimensions);

        if residual_rows == 0 {
            residual_dimensions.resize_with(dimensions, Vec::new);
        } else {
            for _ in 0..dimensions {
                residual_dimensions.push(
                    self.read_fixed_compact_scalar_vec("node.residual_dimension", residual_rows)?,
                );
            }
        }

        Ok(FSEPartitionNodeArchiveRecord {
            id,
            centroid,
            bounds_min,
            bounds_max,
            residual_dimensions,
            cardinality: self.read_var_u64("node.cardinality")?,
            children: self.read_compact_u64_vec("node.children")?,
            is_leaf: self.read_bool("node.is_leaf")?,
        })
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

    fn read_compact_scalar_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<Scalar>, FSEArchiveCodecError> {
        match self.read_u8(field)? {
            COMPACT_SCALAR_VEC_RAW_MODE => self.read_scalar_vec(field),
            COMPACT_SCALAR_VEC_EMPTY_MODE => Ok(Vec::new()),
            COMPACT_SCALAR_VEC_REPEATED_MODE => {
                let length = checked_len(field, self.read_u64(field)?)?;
                let value = self.read_scalar(field)?;

                Ok(vec![value; length])
            }
            mode => Err(FSEArchiveCodecError::InvalidCompactScalarVectorMode { field, mode }),
        }
    }

    fn read_fixed_compact_scalar_vec(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<Vec<Scalar>, FSEArchiveCodecError> {
        match self.read_u8(field)? {
            COMPACT_SCALAR_VEC_RAW_MODE => self.read_fixed_scalar_vec(field, length),
            COMPACT_SCALAR_VEC_EMPTY_MODE => Ok(Vec::new()),
            COMPACT_SCALAR_VEC_REPEATED_MODE => {
                let value = self.read_scalar(field)?;

                Ok(vec![value; length])
            }
            COMPACT_SCALAR_VEC_BYTE_PLANES_MODE => self.read_byte_plane_scalar_vec(field, length),
            COMPACT_SCALAR_VEC_INTEGER_VARINT_MODE => {
                self.read_integer_varint_scalar_vec(field, length)
            }
            mode => Err(FSEArchiveCodecError::InvalidCompactScalarVectorMode { field, mode }),
        }
    }

    fn read_integer_varint_scalar_vec(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<Vec<Scalar>, FSEArchiveCodecError> {
        let mut values = Vec::with_capacity(length);

        for _ in 0..length {
            values.push(unzigzag_i64(self.read_var_u64(field)?) as Scalar);
        }

        Ok(values)
    }

    fn read_byte_plane_scalar_vec(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<Vec<Scalar>, FSEArchiveCodecError> {
        let mut scalar_bytes = vec![[0_u8; 4]; length];

        for lane in 0..size_of::<Scalar>() {
            match self.read_u8(field)? {
                COMPACT_SCALAR_BYTE_PLANE_REPEATED_MODE => {
                    let value = self.read_u8(field)?;

                    for bytes in &mut scalar_bytes {
                        bytes[lane] = value;
                    }
                }
                COMPACT_SCALAR_BYTE_PLANE_RAW_MODE => {
                    let lane_bytes = self.read_exact(field, length)?;

                    for (bytes, value) in scalar_bytes.iter_mut().zip(lane_bytes) {
                        bytes[lane] = *value;
                    }
                }
                mode => {
                    return Err(FSEArchiveCodecError::InvalidCompactScalarBytePlaneMode {
                        field,
                        mode,
                    });
                }
            }
        }

        Ok(scalar_bytes
            .into_iter()
            .map(Scalar::from_le_bytes)
            .collect())
    }

    fn read_fixed_scalar_vec(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<Vec<Scalar>, FSEArchiveCodecError> {
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

    fn read_compact_u64_vec(
        &mut self,
        field: &'static str,
    ) -> Result<Vec<u64>, FSEArchiveCodecError> {
        match self.read_u8(field)? {
            COMPACT_U64_VEC_RAW_MODE => self.read_u64_vec(field),
            COMPACT_U64_VEC_VARINT_MODE => {
                let length = checked_len(field, self.read_u64(field)?)?;
                let mut values = Vec::with_capacity(length);

                for _ in 0..length {
                    values.push(self.read_var_u64(field)?);
                }

                Ok(values)
            }
            COMPACT_U64_VEC_EMPTY_MODE => Ok(Vec::new()),
            mode => Err(FSEArchiveCodecError::InvalidCompactIntegerVectorMode { field, mode }),
        }
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

    fn read_u8(&mut self, field: &'static str) -> Result<u8, FSEArchiveCodecError> {
        let bytes = self.read_exact(field, 1)?;

        Ok(bytes[0])
    }

    fn read_scalar(&mut self, field: &'static str) -> Result<Scalar, FSEArchiveCodecError> {
        let bytes = self.read_exact(field, size_of::<Scalar>())?;

        Ok(Scalar::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))
    }

    fn read_var_u64(&mut self, field: &'static str) -> Result<u64, FSEArchiveCodecError> {
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

        Err(FSEArchiveCodecError::LengthOutOfRange {
            field,
            length: u64::MAX,
        })
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
