//! Archive snapshots for row-mapped FSE indexes.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::build::RowMappedFSEIndex;
use crate::data::RowId;

use super::{FSEArchiveSnapshotError, FSEIndexArchiveSnapshot};

/// Error returned when row-mapped archive snapshot validation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSERowMappedArchiveSnapshotError {
    /// The embedded index snapshot is invalid.
    Index(FSEArchiveSnapshotError),

    /// A row-id record referenced a node outside the index snapshot.
    UnknownNode {
        /// Node id found in the row-id record.
        node_id: u64,
        /// Number of nodes stored by the index snapshot.
        node_count: u64,
    },

    /// A row-id record referenced an internal node.
    InternalNodeMapping {
        /// Internal node id found in the row-id record.
        node_id: u64,
    },

    /// A leaf node had more than one row-id record.
    DuplicateLeafMapping {
        /// Leaf node id with duplicate row-id records.
        node_id: u64,
    },

    /// A leaf node did not have a row-id record.
    MissingLeafMapping {
        /// Leaf node id without a row-id record.
        node_id: u64,
    },

    /// A leaf row-id record did not match the residual row count.
    LeafRowIdCountMismatch {
        /// Leaf node id containing the mismatch.
        node_id: u64,
        /// Number of row identifiers stored by the row-id record.
        row_id_count: u64,
        /// Number of residual rows stored by the leaf node.
        leaf_row_count: u64,
    },

    /// A row identifier appeared more than once in the row mapping.
    DuplicateRowId {
        /// Repeated row identifier.
        row_id: u64,
    },

    /// A numeric archive field cannot be represented by the runtime type.
    ValueOutOfRange {
        /// Field being converted.
        field: &'static str,
        /// Value found in the archive record.
        value: u64,
    },
}

impl fmt::Display for FSERowMappedArchiveSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(error) => error.fmt(formatter),
            Self::UnknownNode { .. } => {
                formatter.write_str("row-id mapping node id must exist in the index snapshot")
            }
            Self::InternalNodeMapping { .. } => {
                formatter.write_str("row-id mappings must reference leaf nodes")
            }
            Self::DuplicateLeafMapping { .. } => {
                formatter.write_str("leaf nodes must have at most one row-id mapping")
            }
            Self::MissingLeafMapping { .. } => {
                formatter.write_str("leaf nodes must have row-id mappings")
            }
            Self::LeafRowIdCountMismatch { .. } => {
                formatter.write_str("leaf row-id count must match residual row count")
            }
            Self::DuplicateRowId { .. } => {
                formatter.write_str("row ids must be unique across the row mapping")
            }
            Self::ValueOutOfRange { .. } => {
                formatter.write_str("row-mapping archive field is outside the runtime range")
            }
        }
    }
}

impl Error for FSERowMappedArchiveSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Index(error) => Some(error),
            Self::UnknownNode { .. }
            | Self::InternalNodeMapping { .. }
            | Self::DuplicateLeafMapping { .. }
            | Self::MissingLeafMapping { .. }
            | Self::LeafRowIdCountMismatch { .. }
            | Self::DuplicateRowId { .. }
            | Self::ValueOutOfRange { .. } => None,
        }
    }
}

impl From<FSEArchiveSnapshotError> for FSERowMappedArchiveSnapshotError {
    fn from(error: FSEArchiveSnapshotError) -> Self {
        Self::Index(error)
    }
}

/// Serializable row-id mapping for one leaf node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FSELeafRowIdArchiveRecord {
    /// Leaf node id.
    pub node_id: u64,

    /// Row identifiers stored by residual row order.
    pub row_ids: Vec<u64>,
}

/// Serializable snapshot of a row-mapped FSE index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FSERowMappedIndexArchiveSnapshot {
    /// Numeric index archive snapshot.
    pub index: FSEIndexArchiveSnapshot,

    /// Row-id records for leaf nodes.
    pub leaf_row_id_records: Vec<FSELeafRowIdArchiveRecord>,
}

impl FSERowMappedIndexArchiveSnapshot {
    /// Creates a row-mapped archive snapshot from a runtime row-mapped index.
    pub fn from_row_mapped_index(
        index: &RowMappedFSEIndex,
    ) -> Result<Self, FSERowMappedArchiveSnapshotError> {
        let index_snapshot = FSEIndexArchiveSnapshot::from_index(index.index())?;
        let mut leaf_row_id_records = Vec::new();

        for node_id in index.index().leaf_node_ids() {
            let row_ids = index.leaf_row_ids(*node_id).ok_or(
                FSERowMappedArchiveSnapshotError::MissingLeafMapping {
                    node_id: *node_id as u64,
                },
            )?;
            leaf_row_id_records.push(FSELeafRowIdArchiveRecord {
                node_id: *node_id as u64,
                row_ids: row_ids.iter().map(|row_id| row_id.value()).collect(),
            });
        }

        let snapshot = Self {
            index: index_snapshot,
            leaf_row_id_records,
        };
        snapshot.validate()?;

        Ok(snapshot)
    }

    /// Validates the row-mapped archive snapshot.
    pub fn validate(&self) -> Result<(), FSERowMappedArchiveSnapshotError> {
        self.index.validate()?;

        let node_count = self.index.manifest.node_count;
        let node_count_usize = checked_usize("node count", node_count)?;
        let mut mapped_leaf_nodes = vec![false; node_count_usize];
        let mut row_ids = HashSet::new();

        for record in &self.leaf_row_id_records {
            if record.node_id >= node_count {
                return Err(FSERowMappedArchiveSnapshotError::UnknownNode {
                    node_id: record.node_id,
                    node_count,
                });
            }

            let node_index = checked_usize("row mapping node id", record.node_id)?;
            let node = &self.index.nodes[node_index];

            if !node.is_leaf {
                return Err(FSERowMappedArchiveSnapshotError::InternalNodeMapping {
                    node_id: record.node_id,
                });
            }

            if mapped_leaf_nodes[node_index] {
                return Err(FSERowMappedArchiveSnapshotError::DuplicateLeafMapping {
                    node_id: record.node_id,
                });
            }
            mapped_leaf_nodes[node_index] = true;

            let leaf_row_count = node.residual_dimensions.first().map_or(0, Vec::len) as u64;
            let row_id_count = record.row_ids.len() as u64;

            if row_id_count != leaf_row_count {
                return Err(FSERowMappedArchiveSnapshotError::LeafRowIdCountMismatch {
                    node_id: record.node_id,
                    row_id_count,
                    leaf_row_count,
                });
            }

            for row_id in &record.row_ids {
                if !row_ids.insert(*row_id) {
                    return Err(FSERowMappedArchiveSnapshotError::DuplicateRowId {
                        row_id: *row_id,
                    });
                }
            }
        }

        for node in &self.index.nodes {
            if node.is_leaf && !mapped_leaf_nodes[checked_usize("node id", node.id)?] {
                return Err(FSERowMappedArchiveSnapshotError::MissingLeafMapping {
                    node_id: node.id,
                });
            }
        }

        Ok(())
    }

    /// Rebuilds a runtime row-mapped index from the archive snapshot.
    pub fn to_row_mapped_index(
        &self,
    ) -> Result<RowMappedFSEIndex, FSERowMappedArchiveSnapshotError> {
        self.validate()?;

        let index = self.index.to_index()?;
        let mut leaf_row_ids_by_node = vec![None; index.node_count()];

        for record in &self.leaf_row_id_records {
            let node_id = checked_usize("row mapping node id", record.node_id)?;
            let row_ids = record
                .row_ids
                .iter()
                .map(|row_id| RowId::new(*row_id))
                .collect();
            leaf_row_ids_by_node[node_id] = Some(row_ids);
        }

        Ok(RowMappedFSEIndex::new(index, leaf_row_ids_by_node))
    }
}

fn checked_usize(
    field: &'static str,
    value: u64,
) -> Result<usize, FSERowMappedArchiveSnapshotError> {
    usize::try_from(value)
        .map_err(|_| FSERowMappedArchiveSnapshotError::ValueOutOfRange { field, value })
}
