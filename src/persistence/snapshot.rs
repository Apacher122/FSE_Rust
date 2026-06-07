//! In-memory archive snapshot records.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::math::{BoundingBox, BoundingBoxError, ResidualBlock, ResidualBlockError, Scalar};
use crate::storage::{FSEIndex, FSEIndexError, PartitionNode};

use super::{FSEArchiveManifest, FSEArchiveManifestError, FSEArchiveSections};

/// Error returned when archive snapshot validation or reconstruction fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveSnapshotError {
    /// The archive manifest is invalid.
    Manifest(FSEArchiveManifestError),

    /// The archive node count does not match the manifest.
    NodeCountMismatch {
        /// Node count recorded by the manifest.
        manifest_node_count: u64,
        /// Number of node records in the snapshot.
        actual_node_count: u64,
    },

    /// The root node record count does not match the manifest.
    RootRecordCountMismatch {
        /// Root node id recorded by the manifest.
        root_node_id: u64,
        /// Record count recorded by the root node.
        root_cardinality: u64,
        /// Record count recorded by the manifest.
        manifest_record_count: u64,
    },

    /// A node id does not match its archive position.
    NodeIdMismatch {
        /// Position of the node record in the archive node list.
        position: usize,
        /// Node id found in the archive record.
        node_id: u64,
    },

    /// A node component used the wrong dimensionality.
    DimensionMismatch {
        /// Node containing the invalid component.
        node_id: u64,
        /// Component with the mismatched dimensionality.
        field: &'static str,
        /// Dimensionality found in the archive record.
        actual_dimensions: usize,
        /// Dimensionality required by the manifest.
        expected_dimensions: usize,
    },

    /// A node centroid contains a non-finite coordinate.
    NonFiniteCentroid {
        /// Node containing the non-finite coordinate.
        node_id: u64,
        /// Dimension containing the non-finite coordinate.
        dimension: usize,
    },

    /// A node bounding box is invalid.
    Bounds {
        /// Node containing the invalid bounding box.
        node_id: u64,
        /// Bounding box validation error.
        source: BoundingBoxError,
    },

    /// A node residual block is invalid.
    Residuals {
        /// Node containing the invalid residual block.
        node_id: u64,
        /// Residual block validation error.
        source: ResidualBlockError,
    },

    /// A leaf node has child references.
    LeafHasChildren {
        /// Leaf node containing child references.
        node_id: u64,
    },

    /// A leaf node stores a different number of residual rows than records.
    LeafCardinalityMismatch {
        /// Leaf node containing the mismatch.
        node_id: u64,
        /// Number of residual rows stored by the node.
        stored_rows: u64,
        /// Number of records declared by the node.
        cardinality: u64,
    },

    /// A node stores more residual rows than its declared record count.
    StoredRowsExceedCardinality {
        /// Node containing the mismatch.
        node_id: u64,
        /// Number of residual rows stored by the node.
        stored_rows: u64,
        /// Number of records declared by the node.
        cardinality: u64,
    },

    /// A child id does not reference a node record in the archive.
    ChildReferenceOutOfRange {
        /// Node containing the child reference.
        node_id: u64,
        /// Child id found in the archive record.
        child_id: u64,
        /// Number of node records in the snapshot.
        node_count: u64,
    },

    /// A numeric archive field cannot be represented by the runtime type.
    ValueOutOfRange {
        /// Field being converted.
        field: &'static str,
        /// Value found in the archive record.
        value: u64,
    },

    /// Runtime index construction failed after archive validation.
    RuntimeIndex(FSEIndexError),
}

impl fmt::Display for FSEArchiveSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(error) => error.fmt(formatter),
            Self::NodeCountMismatch { .. } => {
                formatter.write_str("archive node records must match manifest node count")
            }
            Self::RootRecordCountMismatch { .. } => {
                formatter.write_str("archive root cardinality must match manifest record count")
            }
            Self::NodeIdMismatch { .. } => {
                formatter.write_str("archive node ids must match node list positions")
            }
            Self::DimensionMismatch { .. } => {
                formatter.write_str("archive node component dimensionality is invalid")
            }
            Self::NonFiniteCentroid { .. } => {
                formatter.write_str("archive node centroid values must be finite")
            }
            Self::Bounds { source, .. } => source.fmt(formatter),
            Self::Residuals { source, .. } => source.fmt(formatter),
            Self::LeafHasChildren { .. } => {
                formatter.write_str("archive leaf nodes must not contain child references")
            }
            Self::LeafCardinalityMismatch { .. } => {
                formatter.write_str("archive leaf cardinality must match stored residual rows")
            }
            Self::StoredRowsExceedCardinality { .. } => {
                formatter.write_str("archive residual row count must not exceed node cardinality")
            }
            Self::ChildReferenceOutOfRange { .. } => {
                formatter.write_str("archive child references must point to existing nodes")
            }
            Self::ValueOutOfRange { .. } => {
                formatter.write_str("archive numeric field is outside the runtime range")
            }
            Self::RuntimeIndex(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSEArchiveSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Manifest(error) => Some(error),
            Self::Bounds { source, .. } => Some(source),
            Self::Residuals { source, .. } => Some(source),
            Self::RuntimeIndex(error) => Some(error),
            Self::NodeCountMismatch { .. }
            | Self::RootRecordCountMismatch { .. }
            | Self::NodeIdMismatch { .. }
            | Self::DimensionMismatch { .. }
            | Self::NonFiniteCentroid { .. }
            | Self::LeafHasChildren { .. }
            | Self::LeafCardinalityMismatch { .. }
            | Self::StoredRowsExceedCardinality { .. }
            | Self::ChildReferenceOutOfRange { .. }
            | Self::ValueOutOfRange { .. } => None,
        }
    }
}

impl From<FSEArchiveManifestError> for FSEArchiveSnapshotError {
    fn from(error: FSEArchiveManifestError) -> Self {
        Self::Manifest(error)
    }
}

/// Serializable archive record for a partition node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FSEPartitionNodeArchiveRecord {
    /// Stable node id.
    pub id: u64,

    /// Partition centroid coordinates.
    pub centroid: Vec<Scalar>,

    /// Minimum coordinate value for each bounded-support dimension.
    pub bounds_min: Vec<Scalar>,

    /// Maximum coordinate value for each bounded-support dimension.
    pub bounds_max: Vec<Scalar>,

    /// Residual values grouped by dimension.
    pub residual_dimensions: Vec<Vec<Scalar>>,

    /// Number of logical records represented by the node.
    pub cardinality: u64,

    /// Child node ids.
    pub children: Vec<u64>,

    /// Whether this record represents a terminal leaf partition.
    pub is_leaf: bool,
}

impl FSEPartitionNodeArchiveRecord {
    /// Creates an archive record from a runtime partition node.
    pub fn from_partition_node(node: &PartitionNode) -> Self {
        Self {
            id: node.id as u64,
            centroid: node.centroid.clone(),
            bounds_min: node.bounds.min.clone(),
            bounds_max: node.bounds.max.clone(),
            residual_dimensions: node.residuals.dimensions.clone(),
            cardinality: node.cardinality as u64,
            children: node.children.iter().map(|child| *child as u64).collect(),
            is_leaf: node.is_leaf,
        }
    }

    fn validate(
        &self,
        position: usize,
        expected_dimensions: usize,
        node_count: u64,
    ) -> Result<(), FSEArchiveSnapshotError> {
        if self.id != position as u64 {
            return Err(FSEArchiveSnapshotError::NodeIdMismatch {
                position,
                node_id: self.id,
            });
        }

        validate_dimensions(
            self.id,
            "centroid",
            self.centroid.len(),
            expected_dimensions,
        )?;
        validate_dimensions(
            self.id,
            "bounds_min",
            self.bounds_min.len(),
            expected_dimensions,
        )?;
        validate_dimensions(
            self.id,
            "bounds_max",
            self.bounds_max.len(),
            expected_dimensions,
        )?;
        validate_dimensions(
            self.id,
            "residual_dimensions",
            self.residual_dimensions.len(),
            expected_dimensions,
        )?;

        for (dimension, value) in self.centroid.iter().enumerate() {
            if !value.is_finite() {
                return Err(FSEArchiveSnapshotError::NonFiniteCentroid {
                    node_id: self.id,
                    dimension,
                });
            }
        }

        BoundingBox::try_new(self.bounds_min.clone(), self.bounds_max.clone()).map_err(
            |source| FSEArchiveSnapshotError::Bounds {
                node_id: self.id,
                source,
            },
        )?;
        let residuals =
            ResidualBlock::try_new(self.residual_dimensions.clone()).map_err(|source| {
                FSEArchiveSnapshotError::Residuals {
                    node_id: self.id,
                    source,
                }
            })?;

        let stored_rows = residuals.cardinality() as u64;

        if self.is_leaf && !self.children.is_empty() {
            return Err(FSEArchiveSnapshotError::LeafHasChildren { node_id: self.id });
        }

        if self.is_leaf && stored_rows != self.cardinality {
            return Err(FSEArchiveSnapshotError::LeafCardinalityMismatch {
                node_id: self.id,
                stored_rows,
                cardinality: self.cardinality,
            });
        }

        if stored_rows > self.cardinality {
            return Err(FSEArchiveSnapshotError::StoredRowsExceedCardinality {
                node_id: self.id,
                stored_rows,
                cardinality: self.cardinality,
            });
        }

        for child_id in &self.children {
            if *child_id >= node_count {
                return Err(FSEArchiveSnapshotError::ChildReferenceOutOfRange {
                    node_id: self.id,
                    child_id: *child_id,
                    node_count,
                });
            }
        }

        Ok(())
    }

    fn to_partition_node(&self) -> Result<PartitionNode, FSEArchiveSnapshotError> {
        let id = checked_usize("node id", self.id)?;
        let cardinality = checked_usize("node cardinality", self.cardinality)?;
        let children = self
            .children
            .iter()
            .map(|child| checked_usize("child node id", *child))
            .collect::<Result<Vec<_>, _>>()?;
        let bounds = BoundingBox::try_new(self.bounds_min.clone(), self.bounds_max.clone())
            .map_err(|source| FSEArchiveSnapshotError::Bounds {
                node_id: self.id,
                source,
            })?;
        let residuals =
            ResidualBlock::try_new(self.residual_dimensions.clone()).map_err(|source| {
                FSEArchiveSnapshotError::Residuals {
                    node_id: self.id,
                    source,
                }
            })?;

        Ok(PartitionNode::with_cardinality(
            id,
            self.centroid.clone(),
            bounds,
            residuals,
            cardinality,
            children,
            self.is_leaf,
        ))
    }
}

/// Serializable snapshot of an FSE index archive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FSEIndexArchiveSnapshot {
    /// Archive manifest.
    pub manifest: FSEArchiveManifest,

    /// Partition node records.
    pub nodes: Vec<FSEPartitionNodeArchiveRecord>,
}

impl FSEIndexArchiveSnapshot {
    /// Creates an archive snapshot from a runtime index.
    pub fn from_index(index: &FSEIndex) -> Result<Self, FSEArchiveSnapshotError> {
        Self::from_index_with_sections(index, FSEArchiveSections::empty())
    }

    /// Creates an archive snapshot from a runtime index and explicit section metadata.
    pub fn from_index_with_sections(
        index: &FSEIndex,
        sections: FSEArchiveSections,
    ) -> Result<Self, FSEArchiveSnapshotError> {
        let manifest = FSEArchiveManifest::try_new(
            checked_u32("dimensions", index.dimensions as u64)?,
            index.root_node().cardinality as u64,
            index.nodes.len() as u64,
            index.root as u64,
            sections,
        )?;
        let nodes = index
            .nodes
            .iter()
            .map(FSEPartitionNodeArchiveRecord::from_partition_node)
            .collect();
        let snapshot = Self { manifest, nodes };

        snapshot.validate()?;

        Ok(snapshot)
    }

    /// Validates the archive snapshot.
    pub fn validate(&self) -> Result<(), FSEArchiveSnapshotError> {
        self.manifest.validate()?;

        let actual_node_count = self.nodes.len() as u64;
        if self.manifest.node_count != actual_node_count {
            return Err(FSEArchiveSnapshotError::NodeCountMismatch {
                manifest_node_count: self.manifest.node_count,
                actual_node_count,
            });
        }

        let expected_dimensions = self.manifest.dimensions as usize;
        for (position, node) in self.nodes.iter().enumerate() {
            node.validate(position, expected_dimensions, self.manifest.node_count)?;
        }

        let root_node = &self.nodes[self.manifest.root_node_id as usize];
        if root_node.cardinality != self.manifest.record_count {
            return Err(FSEArchiveSnapshotError::RootRecordCountMismatch {
                root_node_id: self.manifest.root_node_id,
                root_cardinality: root_node.cardinality,
                manifest_record_count: self.manifest.record_count,
            });
        }

        Ok(())
    }

    /// Rebuilds a runtime index from the archive snapshot.
    pub fn to_index(&self) -> Result<FSEIndex, FSEArchiveSnapshotError> {
        self.validate()?;

        let nodes = self
            .nodes
            .iter()
            .map(FSEPartitionNodeArchiveRecord::to_partition_node)
            .collect::<Result<Vec<_>, _>>()?;
        let root = checked_usize("root node id", self.manifest.root_node_id)?;

        FSEIndex::try_new(nodes, root).map_err(FSEArchiveSnapshotError::RuntimeIndex)
    }
}

fn validate_dimensions(
    node_id: u64,
    field: &'static str,
    actual_dimensions: usize,
    expected_dimensions: usize,
) -> Result<(), FSEArchiveSnapshotError> {
    if actual_dimensions != expected_dimensions {
        return Err(FSEArchiveSnapshotError::DimensionMismatch {
            node_id,
            field,
            actual_dimensions,
            expected_dimensions,
        });
    }

    Ok(())
}

fn checked_usize(field: &'static str, value: u64) -> Result<usize, FSEArchiveSnapshotError> {
    usize::try_from(value).map_err(|_| FSEArchiveSnapshotError::ValueOutOfRange { field, value })
}

fn checked_u32(field: &'static str, value: u64) -> Result<u32, FSEArchiveSnapshotError> {
    u32::try_from(value).map_err(|_| FSEArchiveSnapshotError::ValueOutOfRange { field, value })
}
