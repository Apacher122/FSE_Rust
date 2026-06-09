use std::mem::size_of;

use crate::persistence::{
    FSE_ARCHIVE_PAYLOAD_HEADER_VERSION, FSE_ARCHIVE_PAYLOAD_MAGIC, FSEArchivePayloadHeaderError,
    FSEArchivePayloadKind, decode_archive_payload, encode_archive_payload,
};

#[test]
fn archive_payload_round_trips_non_empty_payload() {
    let payload = [1_u8, 2, 3, 4, 5];
    let encoded = encode_archive_payload(FSEArchivePayloadKind::TypedQueryIndex, &payload);

    assert_eq!(
        decode_archive_payload(FSEArchivePayloadKind::TypedQueryIndex, &encoded),
        Ok(payload.to_vec())
    );
}

#[test]
fn archive_payload_reports_length_mismatch_for_truncated_payload() {
    let payload = [1_u8, 2, 3, 4, 5];
    let mut encoded = encode_archive_payload(FSEArchivePayloadKind::TypedRecordBatch, &payload);
    encoded.pop();

    assert_eq!(
        decode_archive_payload(FSEArchivePayloadKind::TypedRecordBatch, &encoded),
        Err(FSEArchivePayloadHeaderError::PayloadLengthMismatch {
            expected: payload.len() as u64,
            actual: payload.len() - 1
        })
    );
}

#[test]
fn archive_payload_reports_checksum_mismatch_for_modified_payload() {
    let payload = [1_u8, 2, 3, 4, 5];
    let mut encoded = encode_archive_payload(FSEArchivePayloadKind::RowMappedIndex, &payload);
    let last = encoded.last_mut().unwrap();
    *last ^= 0xff;

    assert!(matches!(
        decode_archive_payload(FSEArchivePayloadKind::RowMappedIndex, &encoded),
        Err(FSEArchivePayloadHeaderError::PayloadChecksumMismatch { .. })
    ));
}

#[test]
fn archive_payload_reports_unsupported_header_version() {
    let mut encoded = encode_archive_payload(FSEArchivePayloadKind::Index, &[1_u8, 2, 3]);
    let version_start = FSE_ARCHIVE_PAYLOAD_MAGIC.len();
    let version_end = version_start + size_of::<u32>();

    encoded[version_start..version_end].copy_from_slice(&1_u32.to_le_bytes());

    assert_eq!(
        decode_archive_payload(FSEArchivePayloadKind::Index, &encoded),
        Err(
            FSEArchivePayloadHeaderError::UnsupportedPayloadHeaderVersion {
                actual: 1,
                expected: FSE_ARCHIVE_PAYLOAD_HEADER_VERSION
            }
        )
    );
}
