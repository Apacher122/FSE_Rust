//! Append operation metadata for FSE archives.

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

/// Reason an archive rebuild is being planned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEArchiveRebuildReason {
    /// Append records into an existing archive by rebuilding the persisted index.
    Append,
}

/// Error returned when archive rebuild plan metadata is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSEArchiveRebuildPlanMetadataError {
    /// Append operation metadata is invalid.
    Append(FSEArchiveAppendOperationMetadataError),

    /// The plan payload kind did not match the append operation payload kind.
    PayloadKindMismatch {
        /// Payload kind stored on the rebuild plan.
        plan_payload_kind: FSEArchivePayloadKind,

        /// Payload kind stored on the append operation.
        append_payload_kind: FSEArchivePayloadKind,
    },

    /// The plan resulting record count did not match the append operation.
    ResultingRecordCountMismatch {
        /// Resulting record count stored on the rebuild plan.
        plan_resulting_record_count: u64,

        /// Resulting record count stored on the append operation.
        append_resulting_record_count: u64,
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
            Self::PayloadKindMismatch { .. } => {
                formatter.write_str("archive rebuild plan payload kind must match append metadata")
            }
            Self::ResultingRecordCountMismatch { .. } => formatter.write_str(
                "archive rebuild plan resulting record count must match append metadata",
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
            Self::PayloadKindMismatch { .. }
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

    /// Append operation metadata that requires the rebuild.
    pub append: FSEArchiveAppendOperationMetadata,

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
        let plan = Self {
            reason: FSEArchiveRebuildReason::Append,
            payload_kind: append.payload_kind,
            append,
            requires_full_archive_rebuild: true,
            resulting_record_count: append.resulting_record_count,
        };

        plan.validate()?;

        Ok(plan)
    }

    /// Validates archive rebuild plan metadata.
    pub fn validate(&self) -> Result<(), FSEArchiveRebuildPlanMetadataError> {
        self.append.validate()?;

        if self.payload_kind != self.append.payload_kind {
            return Err(FSEArchiveRebuildPlanMetadataError::PayloadKindMismatch {
                plan_payload_kind: self.payload_kind,
                append_payload_kind: self.append.payload_kind,
            });
        }

        if self.resulting_record_count != self.append.resulting_record_count {
            return Err(
                FSEArchiveRebuildPlanMetadataError::ResultingRecordCountMismatch {
                    plan_resulting_record_count: self.resulting_record_count,
                    append_resulting_record_count: self.append.resulting_record_count,
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
