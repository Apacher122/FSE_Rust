use crate::persistence::{
    FSEArchiveAppendOperationMetadata, FSEArchiveAppendOperationMetadataError,
    FSEArchiveCompactionOperationMetadata, FSEArchiveCompactionOperationMetadataError,
    FSEArchiveManifest, FSEArchiveManifestError, FSEArchivePayloadKind,
    FSEArchiveRebuildOperationMetadata, FSEArchiveRebuildPlanMetadata,
    FSEArchiveRebuildPlanMetadataError, FSEArchiveRebuildReason, FSEArchiveSections,
};

#[test]
fn append_operation_metadata_records_count_transition() {
    let metadata =
        FSEArchiveAppendOperationMetadata::try_new(FSEArchivePayloadKind::TypedQueryIndex, 60, 12)
            .unwrap();

    assert_eq!(
        metadata.payload_kind,
        FSEArchivePayloadKind::TypedQueryIndex
    );
    assert_eq!(metadata.base_record_count, 60);
    assert_eq!(metadata.appended_record_count, 12);
    assert_eq!(metadata.resulting_record_count, 72);
    assert_eq!(metadata.validate(), Ok(()));
}

#[test]
fn append_operation_metadata_can_be_created_from_manifest() {
    let manifest = FSEArchiveManifest::try_new(2, 60, 23, 0, FSEArchiveSections::typed()).unwrap();

    let metadata = FSEArchiveAppendOperationMetadata::from_manifest(
        FSEArchivePayloadKind::TypedQueryIndex,
        &manifest,
        5,
    )
    .unwrap();

    assert_eq!(
        metadata.payload_kind,
        FSEArchivePayloadKind::TypedQueryIndex
    );
    assert_eq!(metadata.base_record_count, 60);
    assert_eq!(metadata.appended_record_count, 5);
    assert_eq!(metadata.resulting_record_count, 65);
}

#[test]
fn append_operation_metadata_reports_invalid_manifest() {
    let mut manifest =
        FSEArchiveManifest::try_new(2, 60, 23, 0, FSEArchiveSections::typed()).unwrap();
    manifest.record_count = 0;

    assert_eq!(
        FSEArchiveAppendOperationMetadata::from_manifest(
            FSEArchivePayloadKind::TypedQueryIndex,
            &manifest,
            5
        ),
        Err(FSEArchiveAppendOperationMetadataError::Manifest(
            FSEArchiveManifestError::ZeroRecordCount
        ))
    );
}

#[test]
fn append_operation_metadata_reports_zero_base_record_count() {
    assert_eq!(
        FSEArchiveAppendOperationMetadata::try_new(FSEArchivePayloadKind::TypedQueryIndex, 0, 5),
        Err(FSEArchiveAppendOperationMetadataError::ZeroBaseRecordCount)
    );
}

#[test]
fn append_operation_metadata_reports_zero_appended_record_count() {
    assert_eq!(
        FSEArchiveAppendOperationMetadata::try_new(FSEArchivePayloadKind::TypedQueryIndex, 60, 0),
        Err(FSEArchiveAppendOperationMetadataError::ZeroAppendedRecordCount)
    );
}

#[test]
fn append_operation_metadata_reports_result_count_overflow() {
    assert_eq!(
        FSEArchiveAppendOperationMetadata::try_new(
            FSEArchivePayloadKind::TypedQueryIndex,
            u64::MAX,
            1
        ),
        Err(
            FSEArchiveAppendOperationMetadataError::ResultingRecordCountOverflow {
                base_record_count: u64::MAX,
                appended_record_count: 1
            }
        )
    );
}

#[test]
fn append_operation_metadata_reports_result_count_mismatch() {
    let metadata = FSEArchiveAppendOperationMetadata {
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        base_record_count: 60,
        appended_record_count: 5,
        resulting_record_count: 66,
    };

    assert_eq!(
        metadata.validate(),
        Err(
            FSEArchiveAppendOperationMetadataError::ResultingRecordCountMismatch {
                base_record_count: 60,
                appended_record_count: 5,
                resulting_record_count: 66,
                expected_resulting_record_count: 65
            }
        )
    );
}

#[test]
fn compaction_operation_metadata_records_count_transition() {
    let metadata = FSEArchiveCompactionOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        60,
        15,
        15,
    )
    .unwrap();

    assert_eq!(
        metadata.payload_kind,
        FSEArchivePayloadKind::TypedQueryIndex
    );
    assert_eq!(metadata.base_record_count, 60);
    assert_eq!(metadata.tombstone_count, 15);
    assert_eq!(metadata.removed_record_count, 15);
    assert_eq!(metadata.retained_record_count, 45);
    assert_eq!(metadata.validate(), Ok(()));
}

#[test]
fn compaction_operation_metadata_allows_stale_tombstones() {
    let metadata = FSEArchiveCompactionOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        60,
        3,
        0,
    )
    .unwrap();

    assert_eq!(metadata.base_record_count, 60);
    assert_eq!(metadata.tombstone_count, 3);
    assert_eq!(metadata.removed_record_count, 0);
    assert_eq!(metadata.retained_record_count, 60);
}

#[test]
fn compaction_operation_metadata_can_be_created_from_manifest() {
    let manifest = FSEArchiveManifest::try_new(2, 60, 23, 0, FSEArchiveSections::typed()).unwrap();

    let metadata = FSEArchiveCompactionOperationMetadata::from_manifest(
        FSEArchivePayloadKind::TypedQueryIndex,
        &manifest,
        15,
        12,
    )
    .unwrap();

    assert_eq!(
        metadata.payload_kind,
        FSEArchivePayloadKind::TypedQueryIndex
    );
    assert_eq!(metadata.base_record_count, 60);
    assert_eq!(metadata.tombstone_count, 15);
    assert_eq!(metadata.removed_record_count, 12);
    assert_eq!(metadata.retained_record_count, 48);
}

#[test]
fn compaction_operation_metadata_reports_invalid_manifest() {
    let mut manifest =
        FSEArchiveManifest::try_new(2, 60, 23, 0, FSEArchiveSections::typed()).unwrap();
    manifest.record_count = 0;

    assert_eq!(
        FSEArchiveCompactionOperationMetadata::from_manifest(
            FSEArchivePayloadKind::TypedQueryIndex,
            &manifest,
            5,
            1,
        ),
        Err(FSEArchiveCompactionOperationMetadataError::Manifest(
            FSEArchiveManifestError::ZeroRecordCount
        ))
    );
}

#[test]
fn compaction_operation_metadata_reports_zero_base_record_count() {
    assert_eq!(
        FSEArchiveCompactionOperationMetadata::try_new(
            FSEArchivePayloadKind::TypedQueryIndex,
            0,
            5,
            1,
        ),
        Err(FSEArchiveCompactionOperationMetadataError::ZeroBaseRecordCount)
    );
}

#[test]
fn compaction_operation_metadata_reports_zero_tombstone_count() {
    assert_eq!(
        FSEArchiveCompactionOperationMetadata::try_new(
            FSEArchivePayloadKind::TypedQueryIndex,
            60,
            0,
            1,
        ),
        Err(FSEArchiveCompactionOperationMetadataError::ZeroTombstoneCount)
    );
}

#[test]
fn compaction_operation_metadata_reports_removed_record_count_above_base() {
    assert_eq!(
        FSEArchiveCompactionOperationMetadata::try_new(
            FSEArchivePayloadKind::TypedQueryIndex,
            60,
            61,
            61,
        ),
        Err(
            FSEArchiveCompactionOperationMetadataError::RemovedRecordCountExceedsBase {
                base_record_count: 60,
                removed_record_count: 61
            }
        )
    );
}

#[test]
fn compaction_operation_metadata_reports_empty_retained_record_set() {
    assert_eq!(
        FSEArchiveCompactionOperationMetadata::try_new(
            FSEArchivePayloadKind::TypedQueryIndex,
            60,
            60,
            60,
        ),
        Err(
            FSEArchiveCompactionOperationMetadataError::EmptyRetainedRecordSet {
                base_record_count: 60,
                removed_record_count: 60
            }
        )
    );
}

#[test]
fn compaction_operation_metadata_reports_retained_record_count_mismatch() {
    let metadata = FSEArchiveCompactionOperationMetadata {
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        base_record_count: 60,
        tombstone_count: 15,
        removed_record_count: 15,
        retained_record_count: 44,
    };

    assert_eq!(
        metadata.validate(),
        Err(
            FSEArchiveCompactionOperationMetadataError::RetainedRecordCountMismatch {
                base_record_count: 60,
                removed_record_count: 15,
                retained_record_count: 44,
                expected_retained_record_count: 45
            }
        )
    );
}

#[test]
fn archive_rebuild_plan_metadata_records_append_rebuild_intent() {
    let append =
        FSEArchiveAppendOperationMetadata::try_new(FSEArchivePayloadKind::TypedQueryIndex, 60, 5)
            .unwrap();

    let plan = FSEArchiveRebuildPlanMetadata::for_append(append).unwrap();

    assert_eq!(plan.reason, FSEArchiveRebuildReason::Append);
    assert_eq!(plan.payload_kind, FSEArchivePayloadKind::TypedQueryIndex);
    assert_eq!(
        plan.operation,
        FSEArchiveRebuildOperationMetadata::Append(append)
    );
    assert!(plan.requires_full_archive_rebuild);
    assert_eq!(plan.resulting_record_count, 65);
    assert_eq!(plan.validate(), Ok(()));
}

#[test]
fn archive_rebuild_plan_metadata_records_compaction_rebuild_intent() {
    let compaction = FSEArchiveCompactionOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        60,
        15,
        15,
    )
    .unwrap();

    let plan = FSEArchiveRebuildPlanMetadata::for_compaction(compaction).unwrap();

    assert_eq!(plan.reason, FSEArchiveRebuildReason::Compaction);
    assert_eq!(plan.payload_kind, FSEArchivePayloadKind::TypedQueryIndex);
    assert_eq!(
        plan.operation,
        FSEArchiveRebuildOperationMetadata::Compaction(compaction)
    );
    assert!(plan.requires_full_archive_rebuild);
    assert_eq!(plan.resulting_record_count, 45);
    assert_eq!(plan.validate(), Ok(()));
}

#[test]
fn archive_rebuild_plan_metadata_reports_invalid_append_metadata() {
    let append = FSEArchiveAppendOperationMetadata {
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        base_record_count: 60,
        appended_record_count: 0,
        resulting_record_count: 60,
    };
    let plan = FSEArchiveRebuildPlanMetadata {
        reason: FSEArchiveRebuildReason::Append,
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        operation: FSEArchiveRebuildOperationMetadata::Append(append),
        requires_full_archive_rebuild: true,
        resulting_record_count: 60,
    };

    assert_eq!(
        plan.validate(),
        Err(FSEArchiveRebuildPlanMetadataError::Append(
            FSEArchiveAppendOperationMetadataError::ZeroAppendedRecordCount
        ))
    );
}

#[test]
fn archive_rebuild_plan_metadata_reports_invalid_compaction_metadata() {
    let compaction = FSEArchiveCompactionOperationMetadata {
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        base_record_count: 60,
        tombstone_count: 0,
        removed_record_count: 0,
        retained_record_count: 60,
    };
    let plan = FSEArchiveRebuildPlanMetadata {
        reason: FSEArchiveRebuildReason::Compaction,
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        operation: FSEArchiveRebuildOperationMetadata::Compaction(compaction),
        requires_full_archive_rebuild: true,
        resulting_record_count: 60,
    };

    assert_eq!(
        plan.validate(),
        Err(FSEArchiveRebuildPlanMetadataError::Compaction(
            FSEArchiveCompactionOperationMetadataError::ZeroTombstoneCount
        ))
    );
}

#[test]
fn archive_rebuild_plan_metadata_reports_reason_mismatch() {
    let compaction = FSEArchiveCompactionOperationMetadata::try_new(
        FSEArchivePayloadKind::TypedQueryIndex,
        60,
        15,
        15,
    )
    .unwrap();
    let plan = FSEArchiveRebuildPlanMetadata {
        reason: FSEArchiveRebuildReason::Append,
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        operation: FSEArchiveRebuildOperationMetadata::Compaction(compaction),
        requires_full_archive_rebuild: true,
        resulting_record_count: compaction.retained_record_count,
    };

    assert_eq!(
        plan.validate(),
        Err(FSEArchiveRebuildPlanMetadataError::ReasonMismatch {
            plan_reason: FSEArchiveRebuildReason::Append,
            operation_reason: FSEArchiveRebuildReason::Compaction
        })
    );
}

#[test]
fn archive_rebuild_plan_metadata_reports_payload_kind_mismatch() {
    let append =
        FSEArchiveAppendOperationMetadata::try_new(FSEArchivePayloadKind::TypedQueryIndex, 60, 5)
            .unwrap();
    let plan = FSEArchiveRebuildPlanMetadata {
        reason: FSEArchiveRebuildReason::Append,
        payload_kind: FSEArchivePayloadKind::TypedRecordBatch,
        operation: FSEArchiveRebuildOperationMetadata::Append(append),
        requires_full_archive_rebuild: true,
        resulting_record_count: append.resulting_record_count,
    };

    assert_eq!(
        plan.validate(),
        Err(FSEArchiveRebuildPlanMetadataError::PayloadKindMismatch {
            plan_payload_kind: FSEArchivePayloadKind::TypedRecordBatch,
            operation_payload_kind: FSEArchivePayloadKind::TypedQueryIndex
        })
    );
}

#[test]
fn archive_rebuild_plan_metadata_reports_resulting_record_count_mismatch() {
    let append =
        FSEArchiveAppendOperationMetadata::try_new(FSEArchivePayloadKind::TypedQueryIndex, 60, 5)
            .unwrap();
    let plan = FSEArchiveRebuildPlanMetadata {
        reason: FSEArchiveRebuildReason::Append,
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        operation: FSEArchiveRebuildOperationMetadata::Append(append),
        requires_full_archive_rebuild: true,
        resulting_record_count: 66,
    };

    assert_eq!(
        plan.validate(),
        Err(
            FSEArchiveRebuildPlanMetadataError::ResultingRecordCountMismatch {
                plan_resulting_record_count: 66,
                operation_resulting_record_count: 65
            }
        )
    );
}

#[test]
fn archive_rebuild_plan_metadata_reports_missing_full_rebuild_requirement() {
    let append =
        FSEArchiveAppendOperationMetadata::try_new(FSEArchivePayloadKind::TypedQueryIndex, 60, 5)
            .unwrap();
    let plan = FSEArchiveRebuildPlanMetadata {
        reason: FSEArchiveRebuildReason::Append,
        payload_kind: FSEArchivePayloadKind::TypedQueryIndex,
        operation: FSEArchiveRebuildOperationMetadata::Append(append),
        requires_full_archive_rebuild: false,
        resulting_record_count: append.resulting_record_count,
    };

    assert_eq!(
        plan.validate(),
        Err(
            FSEArchiveRebuildPlanMetadataError::FullArchiveRebuildRequired {
                reason: FSEArchiveRebuildReason::Append
            }
        )
    );
}
