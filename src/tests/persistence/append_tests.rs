use crate::persistence::{
    FSEArchiveAppendOperationMetadata, FSEArchiveAppendOperationMetadataError, FSEArchiveManifest,
    FSEArchiveManifestError, FSEArchivePayloadKind, FSEArchiveSections,
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
