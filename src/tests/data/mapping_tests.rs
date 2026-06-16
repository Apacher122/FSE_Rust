use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSESchema, FSESchemaDimensionMapping,
    FSESchemaDimensionMappingError,
};

#[test]
fn schema_dimension_mapping_accepts_valid_field_dimensions() {
    let schema = crime_schema();
    let mapping = FSESchemaDimensionMapping::new(
        &schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(2, 1),
            FSEDimensionMapping::new(3, 2),
        ],
    );

    assert_eq!(mapping.len(), 3);
    assert!(!mapping.is_empty());
    assert_eq!(mapping.mappings()[0], FSEDimensionMapping::new(0, 0));
    assert_eq!(
        mapping
            .mapping_for_dimension(1)
            .expect("dimension mapping should exist"),
        &FSEDimensionMapping::new(2, 1)
    );
    assert_eq!(mapping.mapping_for_dimension(99), None);
}

#[test]
fn schema_dimension_mapping_allows_field_to_expand_across_dimensions() {
    let schema = crime_schema();
    let mapping = FSESchemaDimensionMapping::new(
        &schema,
        vec![
            FSEDimensionMapping::new(1, 0),
            FSEDimensionMapping::new(1, 1),
        ],
    );

    assert_eq!(
        mapping.mappings_for_field(1),
        vec![
            &FSEDimensionMapping::new(1, 0),
            &FSEDimensionMapping::new(1, 1),
        ]
    );
    assert!(mapping.mappings_for_field(0).is_empty());
}

#[test]
fn schema_dimension_mapping_identity_maps_fields_to_matching_dimensions() {
    let schema = crime_schema();
    let mapping = FSESchemaDimensionMapping::identity(&schema);

    assert_eq!(
        mapping.mappings(),
        &[
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
            FSEDimensionMapping::new(2, 2),
            FSEDimensionMapping::new(3, 3),
        ]
    );
    assert_eq!(
        mapping.mapping_for_dimension(2),
        Some(&FSEDimensionMapping::new(2, 2))
    );
}

#[test]
fn checked_schema_dimension_mapping_reports_empty_mappings() {
    let schema = crime_schema();

    let error = FSESchemaDimensionMapping::try_new(&schema, Vec::new())
        .expect_err("empty dimensional mapping should be rejected");

    assert_eq!(error, FSESchemaDimensionMappingError::EmptyMappings);
    assert_eq!(
        error.to_string(),
        "schema dimensional mapping must contain at least one mapping"
    );
}

#[test]
fn checked_schema_dimension_mapping_reports_field_index_out_of_range() {
    let schema = crime_schema();

    let error = FSESchemaDimensionMapping::try_new(&schema, vec![FSEDimensionMapping::new(4, 0)])
        .expect_err("out-of-range field index should be rejected");

    assert_eq!(
        error,
        FSESchemaDimensionMappingError::FieldIndexOutOfRange {
            field_index: 4,
            field_count: 4,
        }
    );
    assert_eq!(
        error.to_string(),
        "field index 4 is outside schema field count 4"
    );
}

#[test]
fn checked_schema_dimension_mapping_reports_duplicate_dimension() {
    let schema = crime_schema();

    let error = FSESchemaDimensionMapping::try_new(
        &schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 0),
        ],
    )
    .expect_err("duplicate coordinate dimension should be rejected");

    assert_eq!(
        error,
        FSESchemaDimensionMappingError::DuplicateDimension { dimension: 0 }
    );
    assert_eq!(
        error.to_string(),
        "coordinate dimension 0 appears more than once"
    );
}

#[test]
#[should_panic(expected = "schema dimensional mapping must contain at least one mapping")]
fn schema_dimension_mapping_rejects_empty_mappings() {
    let schema = crime_schema();

    let _mapping = FSESchemaDimensionMapping::new(&schema, Vec::new());
}

fn crime_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("case_id", FSEFieldType::Integer, false),
        FSEField::new("category", FSEFieldType::Category, false),
        FSEField::new("latitude", FSEFieldType::Float, false),
        FSEField::new("created_at", FSEFieldType::TimestampMillis, false),
    ])
}
