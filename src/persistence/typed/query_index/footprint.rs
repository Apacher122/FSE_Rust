//! Logical footprint reporting for typed query index archives.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::mem::size_of;
use std::path::{Path, PathBuf};

use crate::encoding::{FSEFieldEncoderMetadata, FSERecordEncoderMetadata};
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, FSE_ARCHIVE_PAYLOAD_MAGIC, FSEArchiveFileOperation,
    FSEArchivePayloadHeaderError, FSEArchivePayloadKind, FSETypedQueryIndexArchiveCodecError,
    FSETypedQueryIndexArchiveSnapshot, decode_archive_payload, encode_row_mapped_archive_snapshot,
};
use crate::query::TypedQueryIndex;

use super::codec::encode_typed_query_index_record_batch_section;

const TYPED_QUERY_INDEX_SECTION_COUNT: u64 = 3;

/// Archive component included in a typed query index logical footprint report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveFootprintComponent {
    /// Typed query index archive file.
    QueryIndex,

    /// Typed record batch append archive file.
    AppendDelta,

    /// Typed row tombstone archive file.
    Tombstones,
}

impl FSETypedQueryIndexArchiveFootprintComponent {
    fn name(self) -> &'static str {
        match self {
            Self::QueryIndex => "typed query index archive",
            Self::AppendDelta => "typed record batch append archive",
            Self::Tombstones => "typed row tombstone archive",
        }
    }
}

/// Error returned when typed query index archive footprint reporting fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveFootprintError {
    /// The path does not use the `.fse` archive extension.
    InvalidFileExtension {
        /// Archive component associated with the path.
        component: FSETypedQueryIndexArchiveFootprintComponent,

        /// Path provided by the caller.
        path: PathBuf,
    },

    /// A filesystem operation failed.
    Io {
        /// Archive component associated with the operation.
        component: FSETypedQueryIndexArchiveFootprintComponent,

        /// Operation that failed.
        operation: FSEArchiveFileOperation,

        /// Path used by the operation.
        path: PathBuf,

        /// Operating-system error kind.
        kind: io::ErrorKind,
    },

    /// The logical archive byte total exceeded `u64::MAX`.
    TotalArchiveByteCountOverflow {
        /// Bytes in the typed query index archive.
        query_index_archive_bytes: u64,

        /// Bytes in the typed record batch append archive.
        append_delta_archive_bytes: u64,

        /// Bytes in the typed row tombstone archive.
        tombstone_archive_bytes: u64,
    },

    /// File-level archive payload decoding failed.
    Payload(FSEArchivePayloadHeaderError),

    /// Typed query index archive encoding or decoding failed.
    TypedQueryIndexArchive(FSETypedQueryIndexArchiveCodecError),

    /// Record encoder metadata byte counting overflowed.
    RecordEncoderMetadataByteCountOverflow,

    /// The typed query index section byte total exceeded `u64::MAX`.
    SectionByteCountOverflow {
        /// Bytes in the file-level archive payload header.
        payload_header_bytes: u64,

        /// Bytes in the embedded row-mapped index section.
        row_mapped_index_section_bytes: u64,

        /// Bytes in the embedded typed record batch section.
        typed_record_batch_section_bytes: u64,

        /// Bytes in the embedded record encoder metadata section.
        record_encoder_metadata_section_bytes: u64,

        /// Bytes used by section length prefixes.
        section_framing_bytes: u64,
    },

    /// The computed section total did not match the archive file length.
    ArchiveByteCountMismatch {
        /// Path provided by the caller.
        path: PathBuf,

        /// Bytes in the archive file.
        file_bytes: u64,

        /// Bytes computed from the decoded archive sections.
        section_total_bytes: u64,
    },
}

impl From<FSEArchivePayloadHeaderError> for FSETypedQueryIndexArchiveFootprintError {
    fn from(error: FSEArchivePayloadHeaderError) -> Self {
        Self::Payload(error)
    }
}

impl From<FSETypedQueryIndexArchiveCodecError> for FSETypedQueryIndexArchiveFootprintError {
    fn from(error: FSETypedQueryIndexArchiveCodecError) -> Self {
        Self::TypedQueryIndexArchive(error)
    }
}

impl fmt::Display for FSETypedQueryIndexArchiveFootprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileExtension { component, .. } => write!(
                formatter,
                "{} footprint path must use the .fse extension",
                component.name()
            ),
            Self::Io {
                component,
                operation,
                ..
            } => match operation {
                FSEArchiveFileOperation::Read => {
                    write!(
                        formatter,
                        "failed to read {} footprint file",
                        component.name()
                    )
                }
                FSEArchiveFileOperation::Write => write!(
                    formatter,
                    "failed to write {} footprint file",
                    component.name()
                ),
            },
            Self::TotalArchiveByteCountOverflow { .. } => {
                formatter.write_str("typed query index logical archive footprint overflowed")
            }
            Self::Payload(error) => error.fmt(formatter),
            Self::TypedQueryIndexArchive(error) => error.fmt(formatter),
            Self::RecordEncoderMetadataByteCountOverflow => {
                formatter.write_str("record encoder metadata footprint overflowed")
            }
            Self::SectionByteCountOverflow { .. } => {
                formatter.write_str("typed query index archive section footprint overflowed")
            }
            Self::ArchiveByteCountMismatch { .. } => formatter
                .write_str("typed query index archive section bytes do not match file bytes"),
        }
    }
}

impl Error for FSETypedQueryIndexArchiveFootprintError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Payload(error) => Some(error),
            Self::TypedQueryIndexArchive(error) => Some(error),
            Self::InvalidFileExtension { .. }
            | Self::Io { .. }
            | Self::TotalArchiveByteCountOverflow { .. }
            | Self::RecordEncoderMetadataByteCountOverflow
            | Self::SectionByteCountOverflow { .. }
            | Self::ArchiveByteCountMismatch { .. } => None,
        }
    }
}

/// Byte footprint for the sections embedded in a typed query index archive file.
///
/// The report breaks the `.fse` file into the payload header, row-mapped index
/// section, typed record batch section, record encoder metadata section, and
/// section length-prefix overhead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSETypedQueryIndexArchiveSectionFootprint {
    /// Bytes in the file-level archive payload header.
    pub payload_header_bytes: u64,

    /// Bytes in the typed query index payload after the file-level header.
    pub typed_query_index_payload_bytes: u64,

    /// Bytes in the embedded row-mapped geometric index section.
    pub row_mapped_index_section_bytes: u64,

    /// Bytes in the embedded typed record batch section.
    pub typed_record_batch_section_bytes: u64,

    /// Bytes in the embedded record encoder metadata section.
    pub record_encoder_metadata_section_bytes: u64,

    /// Bytes used by section length prefixes inside the typed query index payload.
    pub section_framing_bytes: u64,

    /// Total bytes represented by the `.fse` archive file.
    pub total_archive_bytes: u64,
}

impl FSETypedQueryIndexArchiveSectionFootprint {
    /// Creates a section footprint report.
    pub fn try_new(
        payload_header_bytes: u64,
        row_mapped_index_section_bytes: u64,
        typed_record_batch_section_bytes: u64,
        record_encoder_metadata_section_bytes: u64,
        section_framing_bytes: u64,
    ) -> Result<Self, FSETypedQueryIndexArchiveFootprintError> {
        let embedded_section_bytes = row_mapped_index_section_bytes
            .checked_add(typed_record_batch_section_bytes)
            .and_then(|total| total.checked_add(record_encoder_metadata_section_bytes))
            .and_then(|total| total.checked_add(section_framing_bytes))
            .ok_or(
                FSETypedQueryIndexArchiveFootprintError::SectionByteCountOverflow {
                    payload_header_bytes,
                    row_mapped_index_section_bytes,
                    typed_record_batch_section_bytes,
                    record_encoder_metadata_section_bytes,
                    section_framing_bytes,
                },
            )?;
        let total_archive_bytes = payload_header_bytes
            .checked_add(embedded_section_bytes)
            .ok_or(
                FSETypedQueryIndexArchiveFootprintError::SectionByteCountOverflow {
                    payload_header_bytes,
                    row_mapped_index_section_bytes,
                    typed_record_batch_section_bytes,
                    record_encoder_metadata_section_bytes,
                    section_framing_bytes,
                },
            )?;

        Ok(Self {
            payload_header_bytes,
            typed_query_index_payload_bytes: embedded_section_bytes,
            row_mapped_index_section_bytes,
            typed_record_batch_section_bytes,
            record_encoder_metadata_section_bytes,
            section_framing_bytes,
            total_archive_bytes,
        })
    }
}

/// Byte footprint for a logical typed query index archive.
///
/// The report counts the typed query index archive, append-delta archive, and
/// active tombstone archive as one logical archive footprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSETypedQueryIndexArchiveFootprint {
    /// Bytes in the typed query index archive file.
    pub query_index_archive_bytes: u64,

    /// Bytes in the typed record batch append archive file.
    pub append_delta_archive_bytes: u64,

    /// Bytes in the typed row tombstone archive file.
    pub tombstone_archive_bytes: u64,

    /// Combined bytes for all required archive components.
    pub total_archive_bytes: u64,
}

impl FSETypedQueryIndexArchiveFootprint {
    /// Creates a logical archive footprint report without an append-delta archive.
    pub fn try_new(
        query_index_archive_bytes: u64,
        tombstone_archive_bytes: u64,
    ) -> Result<Self, FSETypedQueryIndexArchiveFootprintError> {
        Self::try_new_with_append_delta(query_index_archive_bytes, 0, tombstone_archive_bytes)
    }

    /// Creates a logical archive footprint report with an append-delta archive.
    pub fn try_new_with_append_delta(
        query_index_archive_bytes: u64,
        append_delta_archive_bytes: u64,
        tombstone_archive_bytes: u64,
    ) -> Result<Self, FSETypedQueryIndexArchiveFootprintError> {
        let query_and_append_bytes = query_index_archive_bytes
            .checked_add(append_delta_archive_bytes)
            .ok_or(
                FSETypedQueryIndexArchiveFootprintError::TotalArchiveByteCountOverflow {
                    query_index_archive_bytes,
                    append_delta_archive_bytes,
                    tombstone_archive_bytes,
                },
            )?;
        let total_archive_bytes = query_and_append_bytes
            .checked_add(tombstone_archive_bytes)
            .ok_or(
                FSETypedQueryIndexArchiveFootprintError::TotalArchiveByteCountOverflow {
                    query_index_archive_bytes,
                    append_delta_archive_bytes,
                    tombstone_archive_bytes,
                },
            )?;

        Ok(Self {
            query_index_archive_bytes,
            append_delta_archive_bytes,
            tombstone_archive_bytes,
            total_archive_bytes,
        })
    }

    /// Returns true when the footprint includes an append-delta archive.
    pub fn includes_append_delta_archive(&self) -> bool {
        self.append_delta_archive_bytes > 0
    }

    /// Returns true when the footprint includes a tombstone archive.
    pub fn includes_tombstone_archive(&self) -> bool {
        self.tombstone_archive_bytes > 0
    }
}

/// Reports the file footprint for a typed query index archive.
pub fn typed_query_index_archive_footprint<P>(
    query_index_path: P,
) -> Result<FSETypedQueryIndexArchiveFootprint, FSETypedQueryIndexArchiveFootprintError>
where
    P: AsRef<Path>,
{
    let query_index_archive_bytes = archive_file_len(
        query_index_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
    )?;

    FSETypedQueryIndexArchiveFootprint::try_new(query_index_archive_bytes, 0)
}

/// Reports the embedded section footprint for a runtime typed query index.
pub fn typed_query_index_archive_section_footprint(
    query_index: &TypedQueryIndex,
) -> Result<FSETypedQueryIndexArchiveSectionFootprint, FSETypedQueryIndexArchiveFootprintError> {
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_typed_query_index(query_index)
        .map_err(FSETypedQueryIndexArchiveCodecError::Snapshot)?;

    typed_query_index_archive_section_footprint_from_snapshot(&snapshot)
}

/// Reports the embedded section footprint for a typed query index archive file.
pub fn typed_query_index_archive_file_section_footprint<P>(
    query_index_path: P,
) -> Result<FSETypedQueryIndexArchiveSectionFootprint, FSETypedQueryIndexArchiveFootprintError>
where
    P: AsRef<Path>,
{
    let path = query_index_path.as_ref();
    validate_archive_file_extension(
        path,
        FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
    )?;

    let archive_bytes =
        fs::read(path).map_err(|error| FSETypedQueryIndexArchiveFootprintError::Io {
            component: FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
            operation: FSEArchiveFileOperation::Read,
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;
    let payload = decode_archive_payload(FSEArchivePayloadKind::TypedQueryIndex, &archive_bytes)?;
    let snapshot = FSETypedQueryIndexArchiveSnapshot::from_archive_bytes(&payload)?;
    let footprint = typed_query_index_archive_section_footprint_from_snapshot(&snapshot)?;
    let file_bytes = archive_bytes.len() as u64;

    if footprint.total_archive_bytes != file_bytes {
        return Err(
            FSETypedQueryIndexArchiveFootprintError::ArchiveByteCountMismatch {
                path: path.to_path_buf(),
                file_bytes,
                section_total_bytes: footprint.total_archive_bytes,
            },
        );
    }

    Ok(footprint)
}

/// Reports the logical file footprint for a typed query index archive with append records.
pub fn typed_query_index_archive_with_append_delta_footprint<P, Q>(
    query_index_path: P,
    append_delta_path: Q,
) -> Result<FSETypedQueryIndexArchiveFootprint, FSETypedQueryIndexArchiveFootprintError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_index_archive_bytes = archive_file_len(
        query_index_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
    )?;
    let append_delta_archive_bytes = archive_file_len(
        append_delta_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::AppendDelta,
    )?;

    FSETypedQueryIndexArchiveFootprint::try_new_with_append_delta(
        query_index_archive_bytes,
        append_delta_archive_bytes,
        0,
    )
}

/// Reports the logical file footprint for a typed query index archive with tombstones.
pub fn typed_query_index_archive_with_tombstones_footprint<P, Q>(
    query_index_path: P,
    tombstone_path: Q,
) -> Result<FSETypedQueryIndexArchiveFootprint, FSETypedQueryIndexArchiveFootprintError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let query_index_archive_bytes = archive_file_len(
        query_index_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
    )?;
    let tombstone_archive_bytes = archive_file_len(
        tombstone_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::Tombstones,
    )?;

    FSETypedQueryIndexArchiveFootprint::try_new(query_index_archive_bytes, tombstone_archive_bytes)
}

/// Reports the logical file footprint for all typed query index archive components.
pub fn typed_query_index_archive_with_append_delta_and_tombstones_footprint<P, Q, R>(
    query_index_path: P,
    append_delta_path: Q,
    tombstone_path: R,
) -> Result<FSETypedQueryIndexArchiveFootprint, FSETypedQueryIndexArchiveFootprintError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let query_index_archive_bytes = archive_file_len(
        query_index_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::QueryIndex,
    )?;
    let append_delta_archive_bytes = archive_file_len(
        append_delta_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::AppendDelta,
    )?;
    let tombstone_archive_bytes = archive_file_len(
        tombstone_path.as_ref(),
        FSETypedQueryIndexArchiveFootprintComponent::Tombstones,
    )?;

    FSETypedQueryIndexArchiveFootprint::try_new_with_append_delta(
        query_index_archive_bytes,
        append_delta_archive_bytes,
        tombstone_archive_bytes,
    )
}

fn archive_file_len(
    path: &Path,
    component: FSETypedQueryIndexArchiveFootprintComponent,
) -> Result<u64, FSETypedQueryIndexArchiveFootprintError> {
    validate_archive_file_extension(path, component)?;

    let metadata =
        fs::metadata(path).map_err(|error| FSETypedQueryIndexArchiveFootprintError::Io {
            component,
            operation: FSEArchiveFileOperation::Read,
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;

    Ok(metadata.len())
}

fn validate_archive_file_extension(
    path: &Path,
    component: FSETypedQueryIndexArchiveFootprintComponent,
) -> Result<(), FSETypedQueryIndexArchiveFootprintError> {
    let expected_extension = FSE_ARCHIVE_FILE_EXTENSION.trim_start_matches('.');
    let actual_extension = path.extension().and_then(|extension| extension.to_str());

    if actual_extension == Some(expected_extension) {
        return Ok(());
    }

    Err(
        FSETypedQueryIndexArchiveFootprintError::InvalidFileExtension {
            component,
            path: path.to_path_buf(),
        },
    )
}

fn typed_query_index_archive_section_footprint_from_snapshot(
    snapshot: &FSETypedQueryIndexArchiveSnapshot,
) -> Result<FSETypedQueryIndexArchiveSectionFootprint, FSETypedQueryIndexArchiveFootprintError> {
    snapshot
        .validate()
        .map_err(FSETypedQueryIndexArchiveCodecError::Snapshot)?;

    let row_mapped_index_section_bytes = encode_row_mapped_archive_snapshot(&snapshot.index)
        .map_err(FSETypedQueryIndexArchiveCodecError::IndexCodec)?
        .len() as u64;
    let typed_record_batch_section_bytes =
        encode_typed_query_index_record_batch_section(&snapshot.batch, &snapshot.record_encoder)?
            .len() as u64;
    let record_encoder_metadata_section_bytes =
        record_encoder_metadata_archive_byte_count(&snapshot.record_encoder)?;

    FSETypedQueryIndexArchiveSectionFootprint::try_new(
        archive_payload_header_byte_count(),
        row_mapped_index_section_bytes,
        typed_record_batch_section_bytes,
        record_encoder_metadata_section_bytes,
        typed_query_index_section_framing_byte_count(),
    )
}

fn archive_payload_header_byte_count() -> u64 {
    (FSE_ARCHIVE_PAYLOAD_MAGIC.len()
        + size_of::<u32>()
        + size_of::<u8>()
        + size_of::<u64>()
        + size_of::<u64>()) as u64
}

fn typed_query_index_section_framing_byte_count() -> u64 {
    TYPED_QUERY_INDEX_SECTION_COUNT * size_of::<u64>() as u64
}

fn record_encoder_metadata_archive_byte_count(
    metadata: &FSERecordEncoderMetadata,
) -> Result<u64, FSETypedQueryIndexArchiveFootprintError> {
    let mut total = size_of::<u64>() as u64;

    for field in metadata.fields() {
        total = total
            .checked_add(field_encoder_metadata_archive_byte_count(field)?)
            .ok_or(
                FSETypedQueryIndexArchiveFootprintError::RecordEncoderMetadataByteCountOverflow,
            )?;
    }

    Ok(total)
}

fn field_encoder_metadata_archive_byte_count(
    metadata: &FSEFieldEncoderMetadata,
) -> Result<u64, FSETypedQueryIndexArchiveFootprintError> {
    match metadata {
        FSEFieldEncoderMetadata::Integer
        | FSEFieldEncoderMetadata::Float
        | FSEFieldEncoderMetadata::Boolean
        | FSEFieldEncoderMetadata::TimestampMillis => Ok(size_of::<u8>() as u64),
        FSEFieldEncoderMetadata::CategoryDictionary { categories } => {
            let mut total = size_of::<u8>() as u64 + size_of::<u64>() as u64;

            for category in categories {
                let category_bytes = (size_of::<u64>() + category.len()) as u64;
                total = total.checked_add(category_bytes).ok_or(
                    FSETypedQueryIndexArchiveFootprintError::RecordEncoderMetadataByteCountOverflow,
                )?;
            }

            Ok(total)
        }
    }
}
