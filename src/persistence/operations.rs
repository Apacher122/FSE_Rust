//! Archive operation metadata for FSE archives.

use std::error::Error;
use std::fmt;

use crate::persistence::{FSEArchiveManifest, FSEArchiveManifestError, FSEArchivePayloadKind};

/// Error returned when archive append operation metadata is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveAppendOperationMetadataError {
    /// The source archive manifest is invalid.
    Manifest(FSEArchiveManifestError),

    /// The source archive record count was zero.
    ZeroBaseRecordCount,

    /// The append batch record count was zero.
    ZeroAppendedRecordCount,

    /// Adding the source and append record counts overflowed.
    ResultingRecordCountOverflow {
        /// Number of records in the source archive.
        base_record_count: u64,

        /// Number of records in the append batch.
        appended_record_count: u64,
    },

    /// The resulting record count did not match the source and append counts.
    ResultingRecordCountMismatch {
        /// Number of records in the source archive.
        base_record_count: u64,

        /// Number of records in the append batch.
        appended_record_count: u64,

        /// Resulting record count stored in the metadata.
        resulting_record_count: u64,

        /// Resulting record count computed from the source and append counts.
        expected_resulting_record_count: u64,
    },
}

impl fmt::Display for FSEArchiveAppendOperationMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(error) => error.fmt(formatter),
            Self::ZeroBaseRecordCount => {
                formatter.write_str("append operation base record count must be greater than zero")
            }
            Self::ZeroAppendedRecordCount => {
                formatter.write_str("append operation record count must be greater than zero")
            }
            Self::ResultingRecordCountOverflow { .. } => {
                formatter.write_str("append operation resulting record count overflowed")
            }
            Self::ResultingRecordCountMismatch { .. } => formatter.write_str(
                "append operation resulting record count must equal base records plus appended records",
            ),
        }
    }
}

impl Error for FSEArchiveAppendOperationMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Manifest(error) => Some(error),
            Self::ZeroBaseRecordCount
            | Self::ZeroAppendedRecordCount
            | Self::ResultingRecordCountOverflow { .. }
            | Self::ResultingRecordCountMismatch { .. } => None,
        }
    }
}

impl From<FSEArchiveManifestError> for FSEArchiveAppendOperationMetadataError {
    fn from(error: FSEArchiveManifestError) -> Self {
        Self::Manifest(error)
    }
}

/// Error returned when archive compaction operation metadata is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveCompactionOperationMetadataError {
    /// The source archive manifest is invalid.
    Manifest(FSEArchiveManifestError),

    /// The source archive record count was zero.
    ZeroBaseRecordCount,

    /// The compaction tombstone count was zero.
    ZeroTombstoneCount,

    /// The removed record count exceeded the source record count.
    RemovedRecordCountExceedsBase {
        /// Number of records in the source archive.
        base_record_count: u64,

        /// Number of source records removed by compaction.
        removed_record_count: u64,
    },

    /// Compaction would retain no records.
    EmptyRetainedRecordSet {
        /// Number of records in the source archive.
        base_record_count: u64,

        /// Number of source records removed by compaction.
        removed_record_count: u64,
    },

    /// The retained record count did not match the source and removed counts.
    RetainedRecordCountMismatch {
        /// Number of records in the source archive.
        base_record_count: u64,

        /// Number of source records removed by compaction.
        removed_record_count: u64,

        /// Retained record count stored in the metadata.
        retained_record_count: u64,

        /// Retained record count computed from source and removed counts.
        expected_retained_record_count: u64,
    },
}

impl fmt::Display for FSEArchiveCompactionOperationMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(error) => error.fmt(formatter),
            Self::ZeroBaseRecordCount => {
                formatter.write_str("compaction operation base record count must be greater than zero")
            }
            Self::ZeroTombstoneCount => {
                formatter.write_str("compaction operation tombstone count must be greater than zero")
            }
            Self::RemovedRecordCountExceedsBase { .. } => formatter.write_str(
                "compaction operation removed record count cannot exceed base record count",
            ),
            Self::EmptyRetainedRecordSet { .. } => {
                formatter.write_str("compaction operation retained no records")
            }
            Self::RetainedRecordCountMismatch { .. } => formatter.write_str(
                "compaction operation retained record count must equal base records minus removed records",
            ),
        }
    }
}

impl Error for FSEArchiveCompactionOperationMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Manifest(error) => Some(error),
            Self::ZeroBaseRecordCount
            | Self::ZeroTombstoneCount
            | Self::RemovedRecordCountExceedsBase { .. }
            | Self::EmptyRetainedRecordSet { .. }
            | Self::RetainedRecordCountMismatch { .. } => None,
        }
    }
}

impl From<FSEArchiveManifestError> for FSEArchiveCompactionOperationMetadataError {
    fn from(error: FSEArchiveManifestError) -> Self {
        Self::Manifest(error)
    }
}

/// Reason an archive rebuild is being planned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEArchiveRebuildReason {
    /// Append records into an existing archive by rebuilding the persisted index.
    Append,

    /// Compact tombstoned records by rebuilding the persisted archive.
    Compaction,
}

/// Operation metadata used to plan an archive rebuild.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEArchiveRebuildOperationMetadata {
    /// Append operation metadata.
    Append(FSEArchiveAppendOperationMetadata),

    /// Compaction operation metadata.
    Compaction(FSEArchiveCompactionOperationMetadata),
}

/// Error returned when archive rebuild plan metadata is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveRebuildPlanMetadataError {
    /// Append operation metadata is invalid.
    Append(FSEArchiveAppendOperationMetadataError),

    /// Compaction operation metadata is invalid.
    Compaction(FSEArchiveCompactionOperationMetadataError),

    /// The plan reason did not match the operation metadata.
    ReasonMismatch {
        /// Reason stored on the rebuild plan.
        plan_reason: FSEArchiveRebuildReason,

        /// Reason derived from the rebuild operation.
        operation_reason: FSEArchiveRebuildReason,
    },

    /// The plan payload kind did not match the operation payload kind.
    PayloadKindMismatch {
        /// Payload kind stored on the rebuild plan.
        plan_payload_kind: FSEArchivePayloadKind,

        /// Payload kind stored on the rebuild operation.
        operation_payload_kind: FSEArchivePayloadKind,
    },

    /// The plan resulting record count did not match the operation metadata.
    ResultingRecordCountMismatch {
        /// Resulting record count stored on the rebuild plan.
        plan_resulting_record_count: u64,

        /// Resulting record count derived from the rebuild operation.
        operation_resulting_record_count: u64,
    },

    /// The current plan requires a full archive rebuild.
    FullArchiveRebuildRequired {
        /// Reason for the rebuild plan.
        reason: FSEArchiveRebuildReason,
    },
}

impl fmt::Display for FSEArchiveRebuildPlanMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Append(error) => error.fmt(formatter),
            Self::Compaction(error) => error.fmt(formatter),
            Self::ReasonMismatch { .. } => {
                formatter.write_str("archive rebuild plan reason must match operation metadata")
            }
            Self::PayloadKindMismatch { .. } => formatter
                .write_str("archive rebuild plan payload kind must match operation metadata"),
            Self::ResultingRecordCountMismatch { .. } => formatter.write_str(
                "archive rebuild plan resulting record count must match operation metadata",
            ),
            Self::FullArchiveRebuildRequired { .. } => {
                formatter.write_str("archive rebuild plan requires a full archive rebuild")
            }
        }
    }
}

impl Error for FSEArchiveRebuildPlanMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Append(error) => Some(error),
            Self::Compaction(error) => Some(error),
            Self::ReasonMismatch { .. }
            | Self::PayloadKindMismatch { .. }
            | Self::ResultingRecordCountMismatch { .. }
            | Self::FullArchiveRebuildRequired { .. } => None,
        }
    }
}

impl From<FSEArchiveAppendOperationMetadataError> for FSEArchiveRebuildPlanMetadataError {
    fn from(error: FSEArchiveAppendOperationMetadataError) -> Self {
        Self::Append(error)
    }
}

impl From<FSEArchiveCompactionOperationMetadataError> for FSEArchiveRebuildPlanMetadataError {
    fn from(error: FSEArchiveCompactionOperationMetadataError) -> Self {
        Self::Compaction(error)
    }
}

impl FSEArchiveRebuildOperationMetadata {
    /// Returns the rebuild reason represented by the operation metadata.
    pub fn reason(&self) -> FSEArchiveRebuildReason {
        match self {
            Self::Append(_) => FSEArchiveRebuildReason::Append,
            Self::Compaction(_) => FSEArchiveRebuildReason::Compaction,
        }
    }

    /// Returns the archive payload kind represented by the operation metadata.
    pub fn payload_kind(&self) -> FSEArchivePayloadKind {
        match self {
            Self::Append(metadata) => metadata.payload_kind,
            Self::Compaction(metadata) => metadata.payload_kind,
        }
    }

    /// Returns the record count expected after the rebuild.
    pub fn resulting_record_count(&self) -> u64 {
        match self {
            Self::Append(metadata) => metadata.resulting_record_count,
            Self::Compaction(metadata) => metadata.retained_record_count,
        }
    }

    /// Validates the operation metadata used by the rebuild plan.
    pub fn validate(&self) -> Result<(), FSEArchiveRebuildPlanMetadataError> {
        match self {
            Self::Append(metadata) => metadata.validate().map_err(Into::into),
            Self::Compaction(metadata) => metadata.validate().map_err(Into::into),
        }
    }
}

/// Checked metadata for an archive append operation.
///
/// The metadata records the logical record-count transition before archive
/// rebuild or compaction logic is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSEArchiveAppendOperationMetadata {
    /// Payload kind of the source archive.
    pub payload_kind: FSEArchivePayloadKind,

    /// Number of records in the source archive.
    pub base_record_count: u64,

    /// Number of records in the append batch.
    pub appended_record_count: u64,

    /// Number of records expected after the append operation.
    pub resulting_record_count: u64,
}

/// Checked metadata for an archive compaction operation.
///
/// The metadata records the logical record-count transition when tombstones are
/// applied to a persisted archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSEArchiveCompactionOperationMetadata {
    /// Payload kind of the source archive.
    pub payload_kind: FSEArchivePayloadKind,

    /// Number of records in the source archive.
    pub base_record_count: u64,

    /// Number of tombstones used by the compaction operation.
    pub tombstone_count: u64,

    /// Number of source records removed by compaction.
    pub removed_record_count: u64,

    /// Number of source records retained after compaction.
    pub retained_record_count: u64,
}

impl FSEArchiveCompactionOperationMetadata {
    /// Creates compaction metadata and returns an error when counts are invalid.
    pub fn try_new(
        payload_kind: FSEArchivePayloadKind,
        base_record_count: u64,
        tombstone_count: u64,
        removed_record_count: u64,
    ) -> Result<Self, FSEArchiveCompactionOperationMetadataError> {
        let retained_record_count = base_record_count.saturating_sub(removed_record_count);
        let metadata = Self {
            payload_kind,
            base_record_count,
            tombstone_count,
            removed_record_count,
            retained_record_count,
        };

        metadata.validate()?;

        Ok(metadata)
    }

    /// Creates compaction metadata from a validated archive manifest.
    pub fn from_manifest(
        payload_kind: FSEArchivePayloadKind,
        manifest: &FSEArchiveManifest,
        tombstone_count: u64,
        removed_record_count: u64,
    ) -> Result<Self, FSEArchiveCompactionOperationMetadataError> {
        manifest.validate()?;

        Self::try_new(
            payload_kind,
            manifest.record_count,
            tombstone_count,
            removed_record_count,
        )
    }

    /// Validates compaction operation metadata.
    pub fn validate(&self) -> Result<(), FSEArchiveCompactionOperationMetadataError> {
        if self.base_record_count == 0 {
            return Err(FSEArchiveCompactionOperationMetadataError::ZeroBaseRecordCount);
        }

        if self.tombstone_count == 0 {
            return Err(FSEArchiveCompactionOperationMetadataError::ZeroTombstoneCount);
        }

        let expected_retained_record_count =
            checked_retained_record_count(self.base_record_count, self.removed_record_count)?;

        if expected_retained_record_count == 0 {
            return Err(
                FSEArchiveCompactionOperationMetadataError::EmptyRetainedRecordSet {
                    base_record_count: self.base_record_count,
                    removed_record_count: self.removed_record_count,
                },
            );
        }

        if self.retained_record_count != expected_retained_record_count {
            return Err(
                FSEArchiveCompactionOperationMetadataError::RetainedRecordCountMismatch {
                    base_record_count: self.base_record_count,
                    removed_record_count: self.removed_record_count,
                    retained_record_count: self.retained_record_count,
                    expected_retained_record_count,
                },
            );
        }

        Ok(())
    }
}

impl FSEArchiveAppendOperationMetadata {
    /// Creates append metadata and returns an error when counts are invalid.
    pub fn try_new(
        payload_kind: FSEArchivePayloadKind,
        base_record_count: u64,
        appended_record_count: u64,
    ) -> Result<Self, FSEArchiveAppendOperationMetadataError> {
        let resulting_record_count =
            checked_resulting_record_count(base_record_count, appended_record_count)?;
        let metadata = Self {
            payload_kind,
            base_record_count,
            appended_record_count,
            resulting_record_count,
        };

        metadata.validate()?;

        Ok(metadata)
    }

    /// Creates append metadata from a validated archive manifest.
    pub fn from_manifest(
        payload_kind: FSEArchivePayloadKind,
        manifest: &FSEArchiveManifest,
        appended_record_count: u64,
    ) -> Result<Self, FSEArchiveAppendOperationMetadataError> {
        manifest.validate()?;

        Self::try_new(payload_kind, manifest.record_count, appended_record_count)
    }

    /// Validates append operation metadata.
    pub fn validate(&self) -> Result<(), FSEArchiveAppendOperationMetadataError> {
        if self.base_record_count == 0 {
            return Err(FSEArchiveAppendOperationMetadataError::ZeroBaseRecordCount);
        }

        if self.appended_record_count == 0 {
            return Err(FSEArchiveAppendOperationMetadataError::ZeroAppendedRecordCount);
        }

        let expected_resulting_record_count =
            checked_resulting_record_count(self.base_record_count, self.appended_record_count)?;

        if self.resulting_record_count != expected_resulting_record_count {
            return Err(
                FSEArchiveAppendOperationMetadataError::ResultingRecordCountMismatch {
                    base_record_count: self.base_record_count,
                    appended_record_count: self.appended_record_count,
                    resulting_record_count: self.resulting_record_count,
                    expected_resulting_record_count,
                },
            );
        }

        Ok(())
    }
}

/// Checked metadata for planning an archive rebuild.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSEArchiveRebuildPlanMetadata {
    /// Reason for the rebuild plan.
    pub reason: FSEArchiveRebuildReason,

    /// Payload kind of the archive being rebuilt.
    pub payload_kind: FSEArchivePayloadKind,

    /// Operation metadata that requires the rebuild.
    pub operation: FSEArchiveRebuildOperationMetadata,

    /// Whether the operation requires rebuilding the full archive.
    pub requires_full_archive_rebuild: bool,

    /// Number of records expected after the rebuild.
    pub resulting_record_count: u64,
}

impl FSEArchiveRebuildPlanMetadata {
    /// Creates rebuild plan metadata for an append operation.
    pub fn for_append(
        append: FSEArchiveAppendOperationMetadata,
    ) -> Result<Self, FSEArchiveRebuildPlanMetadataError> {
        let operation = FSEArchiveRebuildOperationMetadata::Append(append);
        let plan = Self {
            reason: operation.reason(),
            payload_kind: operation.payload_kind(),
            operation,
            requires_full_archive_rebuild: true,
            resulting_record_count: operation.resulting_record_count(),
        };

        plan.validate()?;

        Ok(plan)
    }

    /// Creates rebuild plan metadata for a compaction operation.
    pub fn for_compaction(
        compaction: FSEArchiveCompactionOperationMetadata,
    ) -> Result<Self, FSEArchiveRebuildPlanMetadataError> {
        let operation = FSEArchiveRebuildOperationMetadata::Compaction(compaction);
        let plan = Self {
            reason: operation.reason(),
            payload_kind: operation.payload_kind(),
            operation,
            requires_full_archive_rebuild: true,
            resulting_record_count: operation.resulting_record_count(),
        };

        plan.validate()?;

        Ok(plan)
    }

    /// Validates archive rebuild plan metadata.
    pub fn validate(&self) -> Result<(), FSEArchiveRebuildPlanMetadataError> {
        self.operation.validate()?;

        let operation_reason = self.operation.reason();
        if self.reason != operation_reason {
            return Err(FSEArchiveRebuildPlanMetadataError::ReasonMismatch {
                plan_reason: self.reason,
                operation_reason,
            });
        }

        let operation_payload_kind = self.operation.payload_kind();
        if self.payload_kind != operation_payload_kind {
            return Err(FSEArchiveRebuildPlanMetadataError::PayloadKindMismatch {
                plan_payload_kind: self.payload_kind,
                operation_payload_kind,
            });
        }

        let operation_resulting_record_count = self.operation.resulting_record_count();
        if self.resulting_record_count != operation_resulting_record_count {
            return Err(
                FSEArchiveRebuildPlanMetadataError::ResultingRecordCountMismatch {
                    plan_resulting_record_count: self.resulting_record_count,
                    operation_resulting_record_count,
                },
            );
        }

        if !self.requires_full_archive_rebuild {
            return Err(
                FSEArchiveRebuildPlanMetadataError::FullArchiveRebuildRequired {
                    reason: self.reason,
                },
            );
        }

        Ok(())
    }
}

/// Maintenance action selected for an archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEArchiveMaintenanceAction {
    /// No archive maintenance is currently selected.
    NoMaintenance,

    /// Apply pending appended records.
    Append,

    /// Compact tombstoned records.
    Compact,

    /// Rebuild the archive from pending maintenance work.
    Rebuild,
}

/// Reason an archive maintenance action was selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEArchiveMaintenanceReason {
    /// The archive has no pending maintenance work.
    NoPendingMaintenance,

    /// Pending appended records should be applied.
    PendingAppendRecords,

    /// Pending appended records reached the rebuild threshold.
    AppendRebuildThresholdReached,

    /// Tombstone count reached the compaction threshold.
    CompactionTombstoneCountThresholdReached,

    /// Tombstone ratio reached the compaction threshold.
    CompactionTombstoneRatioThresholdReached,

    /// Append and compaction work should be applied by one rebuild.
    AppendAndCompactionThresholdsReached,
}

/// Error returned when archive maintenance policy metadata is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveMaintenanceError {
    /// The source archive record count was zero.
    ZeroBaseRecordCount,

    /// The pending append count overflowed the archive record count.
    ResultingRecordCountOverflow {
        /// Number of records in the source archive.
        base_record_count: u64,

        /// Number of pending appended records.
        pending_append_record_count: u64,
    },

    /// Append rebuild threshold was zero.
    ZeroAppendRebuildRecordCountThreshold,

    /// Compaction tombstone count threshold was zero.
    ZeroCompactionTombstoneCountThreshold,

    /// Compaction tombstone ratio threshold was zero.
    ZeroCompactionTombstoneRatioThreshold,
}

impl fmt::Display for FSEArchiveMaintenanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroBaseRecordCount => {
                formatter.write_str("archive maintenance base record count must be greater than zero")
            }
            Self::ResultingRecordCountOverflow { .. } => {
                formatter.write_str("archive maintenance resulting record count overflowed")
            }
            Self::ZeroAppendRebuildRecordCountThreshold => {
                formatter.write_str("archive maintenance append rebuild threshold must be greater than zero")
            }
            Self::ZeroCompactionTombstoneCountThreshold => formatter.write_str(
                "archive maintenance compaction tombstone count threshold must be greater than zero",
            ),
            Self::ZeroCompactionTombstoneRatioThreshold => formatter.write_str(
                "archive maintenance compaction tombstone ratio threshold must be greater than zero",
            ),
        }
    }
}

impl Error for FSEArchiveMaintenanceError {}

/// Thresholds used to select archive maintenance actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSEArchiveMaintenancePolicy {
    /// Pending append count that selects a full archive rebuild.
    pub append_rebuild_record_count_threshold: u64,

    /// Tombstone count that selects archive compaction.
    pub compaction_tombstone_count_threshold: u64,

    /// Tombstone-to-base-record ratio, in basis points, that selects compaction.
    pub compaction_tombstone_ratio_threshold_basis_points: u64,
}

impl Default for FSEArchiveMaintenancePolicy {
    fn default() -> Self {
        Self {
            append_rebuild_record_count_threshold: 1_024,
            compaction_tombstone_count_threshold: 1_024,
            compaction_tombstone_ratio_threshold_basis_points: 2_500,
        }
    }
}

impl FSEArchiveMaintenancePolicy {
    /// Creates archive maintenance policy metadata.
    pub fn try_new(
        append_rebuild_record_count_threshold: u64,
        compaction_tombstone_count_threshold: u64,
        compaction_tombstone_ratio_threshold_basis_points: u64,
    ) -> Result<Self, FSEArchiveMaintenanceError> {
        let policy = Self {
            append_rebuild_record_count_threshold,
            compaction_tombstone_count_threshold,
            compaction_tombstone_ratio_threshold_basis_points,
        };

        policy.validate()?;

        Ok(policy)
    }

    /// Validates archive maintenance policy metadata.
    pub fn validate(&self) -> Result<(), FSEArchiveMaintenanceError> {
        if self.append_rebuild_record_count_threshold == 0 {
            return Err(FSEArchiveMaintenanceError::ZeroAppendRebuildRecordCountThreshold);
        }

        if self.compaction_tombstone_count_threshold == 0 {
            return Err(FSEArchiveMaintenanceError::ZeroCompactionTombstoneCountThreshold);
        }

        if self.compaction_tombstone_ratio_threshold_basis_points == 0 {
            return Err(FSEArchiveMaintenanceError::ZeroCompactionTombstoneRatioThreshold);
        }

        Ok(())
    }

    /// Evaluates the maintenance action for an archive.
    pub fn evaluate(
        &self,
        input: FSEArchiveMaintenanceInput,
    ) -> Result<FSEArchiveMaintenanceDecision, FSEArchiveMaintenanceError> {
        self.validate()?;
        input.validate()?;

        let tombstone_ratio_basis_points = input.tombstone_ratio_basis_points();
        let has_pending_append = input.pending_append_record_count > 0;
        let append_rebuild_threshold_reached =
            input.pending_append_record_count >= self.append_rebuild_record_count_threshold;
        let compaction_reason = self.compaction_reason(input, tombstone_ratio_basis_points);

        let (action, reason) = match (
            has_pending_append,
            append_rebuild_threshold_reached,
            compaction_reason,
        ) {
            (true, true, Some(_)) | (true, false, Some(_)) => (
                FSEArchiveMaintenanceAction::Rebuild,
                FSEArchiveMaintenanceReason::AppendAndCompactionThresholdsReached,
            ),
            (true, true, None) => (
                FSEArchiveMaintenanceAction::Rebuild,
                FSEArchiveMaintenanceReason::AppendRebuildThresholdReached,
            ),
            (true, false, None) => (
                FSEArchiveMaintenanceAction::Append,
                FSEArchiveMaintenanceReason::PendingAppendRecords,
            ),
            (false, _, Some(reason)) => (FSEArchiveMaintenanceAction::Compact, reason),
            (false, _, None) => (
                FSEArchiveMaintenanceAction::NoMaintenance,
                FSEArchiveMaintenanceReason::NoPendingMaintenance,
            ),
        };

        Ok(FSEArchiveMaintenanceDecision {
            action,
            reason,
            input,
            tombstone_ratio_basis_points,
        })
    }

    fn compaction_reason(
        &self,
        input: FSEArchiveMaintenanceInput,
        tombstone_ratio_basis_points: u64,
    ) -> Option<FSEArchiveMaintenanceReason> {
        if input.tombstone_count >= self.compaction_tombstone_count_threshold {
            return Some(FSEArchiveMaintenanceReason::CompactionTombstoneCountThresholdReached);
        }

        if tombstone_ratio_basis_points >= self.compaction_tombstone_ratio_threshold_basis_points {
            return Some(FSEArchiveMaintenanceReason::CompactionTombstoneRatioThresholdReached);
        }

        None
    }
}

/// Archive state used to evaluate maintenance policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSEArchiveMaintenanceInput {
    /// Number of records in the current archive.
    pub base_record_count: u64,

    /// Number of records waiting to be appended.
    pub pending_append_record_count: u64,

    /// Number of row tombstones waiting to be applied.
    pub tombstone_count: u64,
}

impl FSEArchiveMaintenanceInput {
    /// Creates archive maintenance input metadata.
    pub fn try_new(
        base_record_count: u64,
        pending_append_record_count: u64,
        tombstone_count: u64,
    ) -> Result<Self, FSEArchiveMaintenanceError> {
        let input = Self {
            base_record_count,
            pending_append_record_count,
            tombstone_count,
        };

        input.validate()?;

        Ok(input)
    }

    /// Validates archive maintenance input metadata.
    pub fn validate(&self) -> Result<(), FSEArchiveMaintenanceError> {
        if self.base_record_count == 0 {
            return Err(FSEArchiveMaintenanceError::ZeroBaseRecordCount);
        }

        self.base_record_count
            .checked_add(self.pending_append_record_count)
            .ok_or(FSEArchiveMaintenanceError::ResultingRecordCountOverflow {
                base_record_count: self.base_record_count,
                pending_append_record_count: self.pending_append_record_count,
            })?;

        Ok(())
    }

    /// Returns tombstones per base record in basis points.
    pub fn tombstone_ratio_basis_points(&self) -> u64 {
        ((self.tombstone_count as u128 * 10_000) / self.base_record_count as u128) as u64
    }
}

/// Result of archive maintenance policy evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FSEArchiveMaintenanceDecision {
    /// Selected maintenance action.
    pub action: FSEArchiveMaintenanceAction,

    /// Reason the maintenance action was selected.
    pub reason: FSEArchiveMaintenanceReason,

    /// Archive state used by the policy.
    pub input: FSEArchiveMaintenanceInput,

    /// Tombstone-to-base-record ratio used by the policy.
    pub tombstone_ratio_basis_points: u64,
}

impl FSEArchiveMaintenanceDecision {
    /// Returns whether the decision selects an archive write.
    pub fn requires_archive_write(&self) -> bool {
        self.action != FSEArchiveMaintenanceAction::NoMaintenance
    }
}

fn checked_resulting_record_count(
    base_record_count: u64,
    appended_record_count: u64,
) -> Result<u64, FSEArchiveAppendOperationMetadataError> {
    base_record_count.checked_add(appended_record_count).ok_or(
        FSEArchiveAppendOperationMetadataError::ResultingRecordCountOverflow {
            base_record_count,
            appended_record_count,
        },
    )
}

fn checked_retained_record_count(
    base_record_count: u64,
    removed_record_count: u64,
) -> Result<u64, FSEArchiveCompactionOperationMetadataError> {
    base_record_count.checked_sub(removed_record_count).ok_or(
        FSEArchiveCompactionOperationMetadataError::RemovedRecordCountExceedsBase {
            base_record_count,
            removed_record_count,
        },
    )
}
