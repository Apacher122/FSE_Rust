use crate::data::{
    FSEDatasetMetadata, FSEDatasetMetadataError, FSEDimensionMapping, FSEField, FSEFieldType,
    FSESchema, FSESchemaDimensionMappingError,
};

#[test]
fn dataset_metadata_accepts_valid_schema_mapping_and_record_count() {
    let metadata = FSEDatasetMetadata::new(
        "crime_stats",
        crime_schema(),
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(2, 1),
            FSEDimensionMapping::new(3, 2),
        ],
        128,
    );

    assert_eq!(metadata.name(), "crime_stats");
    assert_eq!(metadata.schema().len(), 4);
    assert_eq!(metadata.dimension_mapping().len(), 3);
    assert_eq!(metadata.record_count(), 128);
}

#[test]
fn dataset_metadata_accepts_zero_record_count() {
    let metadata = FSEDatasetMetadata::new(
        "empty_crime_stats",
        crime_schema(),
        vec![FSEDimensionMapping::new(0, 0)],
        0,
    );

    assert_eq!(metadata.record_count(), 0);
}

#[test]
fn checked_dataset_metadata_reports_empty_name() {
    let error = FSEDatasetMetadata::try_new(
        "",
        crime_schema(),
        vec![FSEDimensionMapping::new(0, 0)],
        128,
    )
    .expect_err("empty dataset name should be rejected");

    assert_eq!(error, FSEDatasetMetadataError::EmptyName);
    assert_eq!(error.to_string(), "dataset name must not be empty");
}

#[test]
fn checked_dataset_metadata_reports_invalid_mapping() {
    let error = FSEDatasetMetadata::try_new(
        "crime_stats",
        crime_schema(),
        vec![FSEDimensionMapping::new(99, 0)],
        128,
    )
    .expect_err("invalid dimensional mapping should be rejected");

    assert_eq!(
        error,
        FSEDatasetMetadataError::DimensionMapping(
            FSESchemaDimensionMappingError::FieldIndexOutOfRange {
                field_index: 99,
                field_count: 4,
            }
        )
    );
    assert_eq!(
        error.to_string(),
        "field index 99 is outside schema field count 4"
    );
}

#[test]
#[should_panic(expected = "dataset name must not be empty")]
fn dataset_metadata_rejects_empty_name() {
    let _metadata = FSEDatasetMetadata::new(
        "",
        crime_schema(),
        vec![FSEDimensionMapping::new(0, 0)],
        128,
    );
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("category", FSEFieldType::Category, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("created_at", FSEFieldType::TimestampMillis, false),
    ])
}
