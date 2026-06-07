//! Versioned archive manifest metadata.

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use serde::{Deserialize, Serialize};

use crate::math::Scalar;

/// Magic string stored in FSE archive manifests.
pub const FSE_ARCHIVE_MAGIC: &str = "FSE_ARCHIVE";

/// Current FSE archive format version.
pub const FSE_ARCHIVE_FORMAT_VERSION: u32 = 1;

/// Canonical file extension for FSE archives.
pub const FSE_ARCHIVE_FILE_EXTENSION: &str = ".fse";

/// Error returned when archive manifest validation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveManifestError {
    /// The manifest magic string does not match the FSE archive identifier.
    InvalidMagic {
        /// Magic string found in the manifest.
        actual: String,
    },

    /// The archive format version is not supported by this runtime.
    UnsupportedFormatVersion {
        /// Version found in the manifest.
        actual: u32,
        /// Version supported by this runtime.
        expected: u32,
    },

    /// The archive file extension does not match the FSE archive extension.
    InvalidFileExtension {
        /// Extension found in the manifest.
        actual: String,
    },

    /// The scalar byte width was zero.
    InvalidScalarSize {
        /// Scalar byte width found in the manifest.
        actual: u32,
    },

    /// The manifest reported zero coordinate dimensions.
    ZeroDimensions,

    /// The manifest reported zero records.
    ZeroRecordCount,

    /// The manifest reported zero partition nodes.
    ZeroNodeCount,

    /// The root node id was outside the node range.
    MissingRootNode {
        /// Root node id found in the manifest.
        root_node_id: u64,
        /// Number of partition nodes reported by the manifest.
        node_count: u64,
    },

    /// Dataset metadata requires schema metadata.
    DatasetMetadataWithoutSchemaMetadata,

    /// Encoder metadata requires schema metadata.
    EncoderMetadataWithoutSchemaMetadata,
}

impl fmt::Display for FSEArchiveManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic { .. } => formatter.write_str("archive magic must identify FSE"),
            Self::UnsupportedFormatVersion { .. } => {
                formatter.write_str("archive format version is not supported")
            }
            Self::InvalidFileExtension { .. } => {
                formatter.write_str("archive file extension must be .fse")
            }
            Self::InvalidScalarSize { .. } => {
                formatter.write_str("archive scalar size must be greater than zero")
            }
            Self::ZeroDimensions => {
                formatter.write_str("archive dimensions must be greater than zero")
            }
            Self::ZeroRecordCount => {
                formatter.write_str("archive record count must be greater than zero")
            }
            Self::ZeroNodeCount => {
                formatter.write_str("archive node count must be greater than zero")
            }
            Self::MissingRootNode { .. } => {
                formatter.write_str("archive root node id must exist in the node list")
            }
            Self::DatasetMetadataWithoutSchemaMetadata => {
                formatter.write_str("archive dataset metadata requires schema metadata")
            }
            Self::EncoderMetadataWithoutSchemaMetadata => {
                formatter.write_str("archive encoder metadata requires schema metadata")
            }
        }
    }
}

impl Error for FSEArchiveManifestError {}

/// Optional logical sections recorded in an FSE archive.
///
/// The section flags describe which metadata blocks are present alongside the
/// partition hierarchy and residual payloads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FSEArchiveSections {
    /// Whether dataset metadata is present.
    pub dataset_metadata: bool,

    /// Whether schema metadata is present.
    pub schema_metadata: bool,

    /// Whether encoder metadata is present.
    pub encoder_metadata: bool,
}

impl FSEArchiveSections {
    /// Creates an empty section set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a section set for typed encoded datasets.
    pub fn typed() -> Self {
        Self {
            dataset_metadata: true,
            schema_metadata: true,
            encoder_metadata: true,
        }
    }
}

/// Versioned metadata for an FSE archive.
///
/// `FSEArchiveManifest` records the stable metadata needed before durable
/// archive save and load routines can interpret hierarchy and residual data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FSEArchiveManifest {
    /// Archive magic string.
    pub magic: String,

    /// Archive format version.
    pub format_version: u32,

    /// Canonical archive file extension.
    pub file_extension: String,

    /// Number of bytes used by one scalar value.
    pub scalar_size_bytes: u32,

    /// Coordinate dimensionality of the persisted index.
    pub dimensions: u32,

    /// Number of logical records represented by the archive.
    pub record_count: u64,

    /// Number of partition nodes in the persisted hierarchy.
    pub node_count: u64,

    /// Root node id in the persisted hierarchy.
    pub root_node_id: u64,

    /// Optional logical metadata sections stored with the archive.
    pub sections: FSEArchiveSections,
}

impl FSEArchiveManifest {
    /// Creates a manifest for the current archive format.
    ///
    /// # Panics
    ///
    /// Panics when dimensions, record count, node count, root node id, or
    /// section metadata are invalid.
    pub fn new(
        dimensions: u32,
        record_count: u64,
        node_count: u64,
        root_node_id: u64,
        sections: FSEArchiveSections,
    ) -> Self {
        Self::try_new(dimensions, record_count, node_count, root_node_id, sections)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a manifest and returns an error when metadata is invalid.
    pub fn try_new(
        dimensions: u32,
        record_count: u64,
        node_count: u64,
        root_node_id: u64,
        sections: FSEArchiveSections,
    ) -> Result<Self, FSEArchiveManifestError> {
        let manifest = Self {
            magic: FSE_ARCHIVE_MAGIC.to_string(),
            format_version: FSE_ARCHIVE_FORMAT_VERSION,
            file_extension: FSE_ARCHIVE_FILE_EXTENSION.to_string(),
            scalar_size_bytes: size_of::<Scalar>() as u32,
            dimensions,
            record_count,
            node_count,
            root_node_id,
            sections,
        };

        manifest.validate()?;

        Ok(manifest)
    }

    /// Validates manifest metadata.
    pub fn validate(&self) -> Result<(), FSEArchiveManifestError> {
        if self.magic != FSE_ARCHIVE_MAGIC {
            return Err(FSEArchiveManifestError::InvalidMagic {
                actual: self.magic.clone(),
            });
        }

        if self.format_version != FSE_ARCHIVE_FORMAT_VERSION {
            return Err(FSEArchiveManifestError::UnsupportedFormatVersion {
                actual: self.format_version,
                expected: FSE_ARCHIVE_FORMAT_VERSION,
            });
        }

        if self.file_extension != FSE_ARCHIVE_FILE_EXTENSION {
            return Err(FSEArchiveManifestError::InvalidFileExtension {
                actual: self.file_extension.clone(),
            });
        }

        if self.scalar_size_bytes == 0 {
            return Err(FSEArchiveManifestError::InvalidScalarSize {
                actual: self.scalar_size_bytes,
            });
        }

        if self.dimensions == 0 {
            return Err(FSEArchiveManifestError::ZeroDimensions);
        }

        if self.record_count == 0 {
            return Err(FSEArchiveManifestError::ZeroRecordCount);
        }

        if self.node_count == 0 {
            return Err(FSEArchiveManifestError::ZeroNodeCount);
        }

        if self.root_node_id >= self.node_count {
            return Err(FSEArchiveManifestError::MissingRootNode {
                root_node_id: self.root_node_id,
                node_count: self.node_count,
            });
        }

        if self.sections.dataset_metadata && !self.sections.schema_metadata {
            return Err(FSEArchiveManifestError::DatasetMetadataWithoutSchemaMetadata);
        }

        if self.sections.encoder_metadata && !self.sections.schema_metadata {
            return Err(FSEArchiveManifestError::EncoderMetadataWithoutSchemaMetadata);
        }

        Ok(())
    }
}
