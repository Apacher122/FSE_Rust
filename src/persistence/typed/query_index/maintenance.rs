//! Maintenance operations for typed query index archives.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::build::FSEBuilder;
use crate::data::FSERecordBatch;
use crate::encoding::FSERecordEncoder;
use crate::persistence::{
    FSEArchiveAppendOperationMetadata, FSEArchiveCompactionOperationMetadata,
    FSEArchiveMaintenanceAction, FSEArchiveMaintenanceDecision, FSEArchiveMaintenanceError,
    FSEArchiveMaintenanceInput, FSEArchiveMaintenancePolicy, FSEArchivePayloadKind,
    FSEArchiveRebuildPlanMetadata,
};
use crate::query::TypedQueryIndex;

use super::super::tombstone::{
    FSETypedRowTombstoneArchiveError, load_typed_row_tombstone_archive_file,
    save_typed_row_tombstone_archive_file,
};
use super::{
    FSETombstonedTypedQueryIndex, FSETypedQueryIndexArchiveAppendResult,
    FSETypedQueryIndexArchiveCompactionError, FSETypedQueryIndexArchiveCompactionResult,
    FSETypedQueryIndexArchiveError, compact_tombstoned_typed_query_index,
    compact_typed_query_index_archive_file, load_typed_query_index_archive_file,
    save_typed_query_index_archive_file,
};

/// Error returned when typed query index archive maintenance fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FSETypedQueryIndexArchiveMaintenanceError {
    /// Loading the typed query index archive failed.
    LoadIndex(FSETypedQueryIndexArchiveError),

    /// Loading the typed row tombstone archive failed.
    LoadTombstones(FSETypedRowTombstoneArchiveError),

    /// Archive maintenance policy evaluation failed.
    Policy(FSEArchiveMaintenanceError),

    /// Applying appended typed records failed.
    Append(FSETypedQueryIndexArchiveError),

    /// Compacting typed row tombstones failed.
    Compaction(FSETypedQueryIndexArchiveCompactionError),
}

impl fmt::Display for FSETypedQueryIndexArchiveMaintenanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadIndex(error) => error.fmt(formatter),
            Self::LoadTombstones(error) => error.fmt(formatter),
            Self::Policy(error) => error.fmt(formatter),
            Self::Append(error) => error.fmt(formatter),
            Self::Compaction(error) => error.fmt(formatter),
        }
    }
}

impl Error for FSETypedQueryIndexArchiveMaintenanceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LoadIndex(error) => Some(error),
            Self::LoadTombstones(error) => Some(error),
            Self::Policy(error) => Some(error),
            Self::Append(error) => Some(error),
            Self::Compaction(error) => Some(error),
        }
    }
}

/// Result returned after applying typed query index archive maintenance.
#[derive(Clone, Debug, PartialEq)]
pub struct FSETypedQueryIndexArchiveMaintenanceResult {
    /// Maintenance decision selected for the archive.
    pub decision: FSEArchiveMaintenanceDecision,

    /// Typed query index stored after maintenance completes.
    pub query_index: TypedQueryIndex,

    /// Append result when appended records were applied.
    pub append_result: Option<FSETypedQueryIndexArchiveAppendResult>,

    /// Compaction result when tombstones were applied.
    pub compaction_result: Option<FSETypedQueryIndexArchiveCompactionResult>,
}

/// Applies archive maintenance policy to a typed query index `.fse` archive file.
pub fn maintain_typed_query_index_archive_file<P, Q>(
    query_index_path: P,
    tombstone_path: Q,
    appended: Option<&FSERecordBatch>,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
    policy: &FSEArchiveMaintenancePolicy,
) -> Result<FSETypedQueryIndexArchiveMaintenanceResult, FSETypedQueryIndexArchiveMaintenanceError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    policy
        .validate()
        .map_err(FSETypedQueryIndexArchiveMaintenanceError::Policy)?;

    let query_index_path = query_index_path.as_ref();
    let tombstone_path = tombstone_path.as_ref();
    let base = load_typed_query_index_archive_file(query_index_path)
        .map_err(FSETypedQueryIndexArchiveMaintenanceError::LoadIndex)?;
    let tombstones = load_typed_row_tombstone_archive_file(tombstone_path)
        .map_err(FSETypedQueryIndexArchiveMaintenanceError::LoadTombstones)?;
    let input = FSEArchiveMaintenanceInput::try_new(
        base.batch().len() as u64,
        appended.map_or(0, |batch| batch.len() as u64),
        tombstones.len() as u64,
    )
    .map_err(FSETypedQueryIndexArchiveMaintenanceError::Policy)?;
    let decision = policy
        .evaluate(input)
        .map_err(FSETypedQueryIndexArchiveMaintenanceError::Policy)?;
    let mut query_index = base;
    let mut append_result = None;
    let mut compaction_result = None;

    match decision.action {
        FSEArchiveMaintenanceAction::NoMaintenance => {}
        FSEArchiveMaintenanceAction::Append => {
            if let Some(appended) = appended {
                let result = super::append_typed_query_index_archive_file(
                    query_index_path,
                    appended,
                    encoder,
                    builder,
                )
                .map_err(FSETypedQueryIndexArchiveMaintenanceError::Append)?;
                query_index = result.query_index.clone();
                append_result = Some(result);
            }
        }
        FSEArchiveMaintenanceAction::Compact => {
            let result = compact_typed_query_index_archive_file(
                query_index_path,
                tombstone_path,
                encoder,
                builder,
            )
            .map_err(FSETypedQueryIndexArchiveMaintenanceError::Compaction)?;
            query_index = result.compaction.query_index.clone();
            compaction_result = Some(result);
        }
        FSEArchiveMaintenanceAction::Rebuild => match (appended, tombstones.is_empty()) {
            (Some(appended), false) => {
                let result = rebuild_appended_and_compacted_typed_query_index_archive_file(
                    query_index_path,
                    tombstone_path,
                    &query_index,
                    tombstones,
                    appended,
                    encoder,
                    builder,
                )?;
                query_index = result.query_index;
                append_result = Some(result.append_result);
                compaction_result = Some(result.compaction_result);
            }
            (Some(appended), true) => {
                let result = super::append_typed_query_index_archive_file(
                    query_index_path,
                    appended,
                    encoder,
                    builder,
                )
                .map_err(FSETypedQueryIndexArchiveMaintenanceError::Append)?;
                query_index = result.query_index.clone();
                append_result = Some(result);
            }
            (None, false) => {
                let result = compact_typed_query_index_archive_file(
                    query_index_path,
                    tombstone_path,
                    encoder,
                    builder,
                )
                .map_err(FSETypedQueryIndexArchiveMaintenanceError::Compaction)?;
                query_index = result.compaction.query_index.clone();
                compaction_result = Some(result);
            }
            (None, true) => {}
        },
    }

    Ok(FSETypedQueryIndexArchiveMaintenanceResult {
        decision,
        query_index,
        append_result,
        compaction_result,
    })
}

struct CombinedArchiveMaintenanceResult {
    query_index: TypedQueryIndex,
    append_result: FSETypedQueryIndexArchiveAppendResult,
    compaction_result: FSETypedQueryIndexArchiveCompactionResult,
}

fn rebuild_appended_and_compacted_typed_query_index_archive_file(
    query_index_path: &Path,
    tombstone_path: &Path,
    base: &TypedQueryIndex,
    tombstones: Vec<crate::data::RowId>,
    appended: &FSERecordBatch,
    encoder: &impl FSERecordEncoder,
    builder: &FSEBuilder,
) -> Result<CombinedArchiveMaintenanceResult, FSETypedQueryIndexArchiveMaintenanceError> {
    let appended_index = base
        .try_append(appended, encoder, builder)
        .map_err(|error| {
            FSETypedQueryIndexArchiveMaintenanceError::Append(
                FSETypedQueryIndexArchiveError::Append(error),
            )
        })?;
    let append_metadata = FSEArchiveAppendOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        base.batch().len() as u64,
        appended.len() as u64,
    )
    .map_err(|error| {
        FSETypedQueryIndexArchiveMaintenanceError::Append(
            FSETypedQueryIndexArchiveError::AppendMetadata(error),
        )
    })?;
    let append_rebuild_plan =
        FSEArchiveRebuildPlanMetadata::for_append(append_metadata).map_err(|error| {
            FSETypedQueryIndexArchiveMaintenanceError::Append(
                FSETypedQueryIndexArchiveError::RebuildPlan(error),
            )
        })?;
    let append_result = FSETypedQueryIndexArchiveAppendResult {
        append_metadata,
        rebuild_plan: append_rebuild_plan,
        query_index: appended_index.clone(),
    };
    let tombstoned = FSETombstonedTypedQueryIndex::from_row_ids(appended_index, tombstones);
    let cleared_tombstone_count = tombstoned.tombstones().len();
    let compaction =
        compact_tombstoned_typed_query_index(&tombstoned, encoder, builder).map_err(|error| {
            FSETypedQueryIndexArchiveMaintenanceError::Compaction(
                FSETypedQueryIndexArchiveCompactionError::Compaction(error),
            )
        })?;
    let compaction_metadata = FSEArchiveCompactionOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        compaction.base_record_count as u64,
        compaction.tombstone_count as u64,
        compaction.removed_record_count as u64,
    )
    .map_err(|error| {
        FSETypedQueryIndexArchiveMaintenanceError::Compaction(
            FSETypedQueryIndexArchiveCompactionError::CompactionMetadata(error),
        )
    })?;
    let compaction_rebuild_plan =
        FSEArchiveRebuildPlanMetadata::for_compaction(compaction_metadata).map_err(|error| {
            FSETypedQueryIndexArchiveMaintenanceError::Compaction(
                FSETypedQueryIndexArchiveCompactionError::RebuildPlan(error),
            )
        })?;
    let query_index = compaction.query_index.clone();

    save_typed_query_index_archive_file(query_index_path, &query_index).map_err(|error| {
        FSETypedQueryIndexArchiveMaintenanceError::Compaction(
            FSETypedQueryIndexArchiveCompactionError::SaveIndex(error),
        )
    })?;
    save_typed_row_tombstone_archive_file(tombstone_path, &[]).map_err(|error| {
        FSETypedQueryIndexArchiveMaintenanceError::Compaction(
            FSETypedQueryIndexArchiveCompactionError::SaveTombstones(error),
        )
    })?;

    Ok(CombinedArchiveMaintenanceResult {
        query_index,
        append_result,
        compaction_result: FSETypedQueryIndexArchiveCompactionResult {
            compaction,
            compaction_metadata,
            rebuild_plan: compaction_rebuild_plan,
            cleared_tombstone_count,
            remaining_tombstone_count: 0,
        },
    })
}
