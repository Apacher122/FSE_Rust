use crate::persistence::{
    FSEArchiveAppendOperationMetadata, FSEArchiveAppendOperationMetadataError,
    FSEArchiveCompactionOperationMetadata, FSEArchiveCompactionOperationMetadataError,
    FSEArchiveMaintenanceAction, FSEArchiveMaintenanceDecision, FSEArchiveMaintenanceError,
    FSEArchiveMaintenanceInput, FSEArchiveMaintenancePolicy, FSEArchiveMaintenanceReason,
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

#[test]
fn archive_maintenance_policy_default_is_valid() {
    let policy = FSEArchiveMaintenancePolicy::default();

    assert_eq!(policy.append_rebuild_record_count_threshold, 1_024);
    assert_eq!(policy.compaction_tombstone_count_threshold, 1_024);
    assert_eq!(
        policy.compaction_tombstone_ratio_threshold_basis_points,
        2_500
    );
    assert_eq!(policy.validate(), Ok(()));
}

#[test]
fn archive_maintenance_policy_reports_zero_append_rebuild_threshold() {
    assert_eq!(
        FSEArchiveMaintenancePolicy::try_new(0, 1_024, 2_500),
        Err(FSEArchiveMaintenanceError::ZeroAppendRebuildRecordCountThreshold)
    );
}

#[test]
fn archive_maintenance_policy_reports_zero_compaction_count_threshold() {
    assert_eq!(
        FSEArchiveMaintenancePolicy::try_new(1_024, 0, 2_500),
        Err(FSEArchiveMaintenanceError::ZeroCompactionTombstoneCountThreshold)
    );
}

#[test]
fn archive_maintenance_policy_reports_zero_compaction_ratio_threshold() {
    assert_eq!(
        FSEArchiveMaintenancePolicy::try_new(1_024, 1_024, 0),
        Err(FSEArchiveMaintenanceError::ZeroCompactionTombstoneRatioThreshold)
    );
}

#[test]
fn archive_maintenance_input_reports_zero_base_record_count() {
    assert_eq!(
        FSEArchiveMaintenanceInput::try_new(0, 1, 0),
        Err(FSEArchiveMaintenanceError::ZeroBaseRecordCount)
    );
}

#[test]
fn archive_maintenance_input_reports_append_record_count_overflow() {
    assert_eq!(
        FSEArchiveMaintenanceInput::try_new(u64::MAX, 1, 0),
        Err(FSEArchiveMaintenanceError::ResultingRecordCountOverflow {
            base_record_count: u64::MAX,
            pending_append_record_count: 1
        })
    );
}

#[test]
fn archive_maintenance_input_reports_tombstone_ratio_basis_points() {
    let input = FSEArchiveMaintenanceInput::try_new(200, 0, 50).unwrap();

    assert_eq!(input.tombstone_ratio_basis_points(), 2_500);
}

#[test]
fn archive_maintenance_policy_selects_no_maintenance_for_clean_archive() {
    let policy = FSEArchiveMaintenancePolicy::try_new(100, 20, 2_500).unwrap();
    let input = FSEArchiveMaintenanceInput::try_new(1_000, 0, 0).unwrap();

    let decision = policy.evaluate(input).unwrap();

    assert_eq!(
        decision,
        FSEArchiveMaintenanceDecision {
            action: FSEArchiveMaintenanceAction::NoMaintenance,
            reason: FSEArchiveMaintenanceReason::NoPendingMaintenance,
            input,
            tombstone_ratio_basis_points: 0
        }
    );
    assert!(!decision.requires_archive_write());
}

#[test]
fn archive_maintenance_policy_selects_append_for_pending_records_below_rebuild_threshold() {
    let policy = FSEArchiveMaintenancePolicy::try_new(100, 20, 2_500).unwrap();
    let input = FSEArchiveMaintenanceInput::try_new(1_000, 10, 0).unwrap();

    let decision = policy.evaluate(input).unwrap();

    assert_eq!(decision.action, FSEArchiveMaintenanceAction::Append);
    assert_eq!(
        decision.reason,
        FSEArchiveMaintenanceReason::PendingAppendRecords
    );
    assert!(decision.requires_archive_write());
}

#[test]
fn archive_maintenance_policy_selects_rebuild_for_pending_records_at_threshold() {
    let policy = FSEArchiveMaintenancePolicy::try_new(100, 20, 2_500).unwrap();
    let input = FSEArchiveMaintenanceInput::try_new(1_000, 100, 0).unwrap();

    let decision = policy.evaluate(input).unwrap();

    assert_eq!(decision.action, FSEArchiveMaintenanceAction::Rebuild);
    assert_eq!(
        decision.reason,
        FSEArchiveMaintenanceReason::AppendRebuildThresholdReached
    );
}

#[test]
fn archive_maintenance_policy_selects_compaction_for_tombstone_count_threshold() {
    let policy = FSEArchiveMaintenancePolicy::try_new(100, 20, 9_000).unwrap();
    let input = FSEArchiveMaintenanceInput::try_new(1_000, 0, 20).unwrap();

    let decision = policy.evaluate(input).unwrap();

    assert_eq!(decision.action, FSEArchiveMaintenanceAction::Compact);
    assert_eq!(
        decision.reason,
        FSEArchiveMaintenanceReason::CompactionTombstoneCountThresholdReached
    );
    assert_eq!(decision.tombstone_ratio_basis_points, 200);
}

#[test]
fn archive_maintenance_policy_selects_compaction_for_tombstone_ratio_threshold() {
    let policy = FSEArchiveMaintenancePolicy::try_new(100, 500, 2_500).unwrap();
    let input = FSEArchiveMaintenanceInput::try_new(1_000, 0, 250).unwrap();

    let decision = policy.evaluate(input).unwrap();

    assert_eq!(decision.action, FSEArchiveMaintenanceAction::Compact);
    assert_eq!(
        decision.reason,
        FSEArchiveMaintenanceReason::CompactionTombstoneRatioThresholdReached
    );
    assert_eq!(decision.tombstone_ratio_basis_points, 2_500);
}

#[test]
fn archive_maintenance_policy_selects_rebuild_for_append_and_compaction_work() {
    let policy = FSEArchiveMaintenancePolicy::try_new(100, 20, 2_500).unwrap();
    let input = FSEArchiveMaintenanceInput::try_new(1_000, 10, 250).unwrap();

    let decision = policy.evaluate(input).unwrap();

    assert_eq!(decision.action, FSEArchiveMaintenanceAction::Rebuild);
    assert_eq!(
        decision.reason,
        FSEArchiveMaintenanceReason::AppendAndCompactionThresholdsReached
    );
    assert_eq!(decision.tombstone_ratio_basis_points, 2_500);
}
