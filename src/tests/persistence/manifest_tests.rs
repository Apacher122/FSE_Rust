use std::mem::size_of;

use crate::math::Scalar;
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, FSE_ARCHIVE_FORMAT_VERSION, FSE_ARCHIVE_MAGIC, FSEArchiveManifest,
    FSEArchiveManifestError, FSEArchiveSections,
};

fn test_manifest() -> FSEArchiveManifest {
    FSEArchiveManifest::new(2, 60, 23, 0, FSEArchiveSections::empty())
}

#[test]
fn archive_manifest_uses_current_format_metadata() {
    let manifest = test_manifest();

    assert_eq!(manifest.magic, FSE_ARCHIVE_MAGIC);
    assert_eq!(manifest.format_version, FSE_ARCHIVE_FORMAT_VERSION);
    assert_eq!(manifest.file_extension, FSE_ARCHIVE_FILE_EXTENSION);
    assert_eq!(manifest.file_extension, ".fse");
    assert_eq!(manifest.scalar_size_bytes, size_of::<Scalar>() as u32);
    assert_eq!(manifest.dimensions, 2);
    assert_eq!(manifest.record_count, 60);
    assert_eq!(manifest.node_count, 23);
    assert_eq!(manifest.root_node_id, 0);
    assert_eq!(manifest.sections, FSEArchiveSections::empty());
}

#[test]
fn archive_manifest_accepts_typed_section_metadata() {
    let manifest = FSEArchiveManifest::new(4, 128, 31, 0, FSEArchiveSections::typed());

    assert!(manifest.sections.dataset_metadata);
    assert!(manifest.sections.schema_metadata);
    assert!(manifest.sections.encoder_metadata);
    assert!(manifest.validate().is_ok());
}

#[test]
fn archive_manifest_reports_invalid_magic() {
    let mut manifest = test_manifest();
    manifest.magic = "OTHER".to_string();

    assert_eq!(
        manifest.validate(),
        Err(FSEArchiveManifestError::InvalidMagic {
            actual: "OTHER".to_string()
        })
    );
}

#[test]
fn archive_manifest_reports_unsupported_format_version() {
    let mut manifest = test_manifest();
    manifest.format_version = FSE_ARCHIVE_FORMAT_VERSION + 1;

    assert_eq!(
        manifest.validate(),
        Err(FSEArchiveManifestError::UnsupportedFormatVersion {
            actual: FSE_ARCHIVE_FORMAT_VERSION + 1,
            expected: FSE_ARCHIVE_FORMAT_VERSION
        })
    );
}

#[test]
fn archive_manifest_reports_invalid_file_extension() {
    let mut manifest = test_manifest();
    manifest.file_extension = "fse".to_string();

    assert_eq!(
        manifest.validate(),
        Err(FSEArchiveManifestError::InvalidFileExtension {
            actual: "fse".to_string()
        })
    );
}

#[test]
fn archive_manifest_reports_zero_required_counts() {
    assert_eq!(
        FSEArchiveManifest::try_new(0, 60, 23, 0, FSEArchiveSections::empty()),
        Err(FSEArchiveManifestError::ZeroDimensions)
    );
    assert_eq!(
        FSEArchiveManifest::try_new(2, 0, 23, 0, FSEArchiveSections::empty()),
        Err(FSEArchiveManifestError::ZeroRecordCount)
    );
    assert_eq!(
        FSEArchiveManifest::try_new(2, 60, 0, 0, FSEArchiveSections::empty()),
        Err(FSEArchiveManifestError::ZeroNodeCount)
    );
}

#[test]
fn archive_manifest_reports_missing_root_node() {
    assert_eq!(
        FSEArchiveManifest::try_new(2, 60, 23, 23, FSEArchiveSections::empty()),
        Err(FSEArchiveManifestError::MissingRootNode {
            root_node_id: 23,
            node_count: 23
        })
    );
}

#[test]
fn archive_manifest_requires_schema_metadata_for_dependent_sections() {
    let dataset_without_schema = FSEArchiveSections {
        dataset_metadata: true,
        schema_metadata: false,
        encoder_metadata: false,
    };
    let encoder_without_schema = FSEArchiveSections {
        dataset_metadata: false,
        schema_metadata: false,
        encoder_metadata: true,
    };

    assert_eq!(
        FSEArchiveManifest::try_new(2, 60, 23, 0, dataset_without_schema),
        Err(FSEArchiveManifestError::DatasetMetadataWithoutSchemaMetadata)
    );
    assert_eq!(
        FSEArchiveManifest::try_new(2, 60, 23, 0, encoder_without_schema),
        Err(FSEArchiveManifestError::EncoderMetadataWithoutSchemaMetadata)
    );
}
