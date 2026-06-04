//! Recursive index construction.

use crate::build::builder::acceptance::accepts_split_quality;
use crate::build::builder::config::BuildConfig;
use crate::build::builder::types::{
    AcceptedStructuralSplit, BuildCheckedError, BuildInputError, BuildValidationError,
    ValidatedFSEIndex,
};
use crate::build::splitter::best_structural_split;
use crate::build::{index_validation_diagnostics, validate_index};
use crate::encoding::EncodedRecordBatch;
use crate::math::{Vector, try_compute_centroid};
use crate::storage::{FSEIndex, PartitionNode};

/// Builder for constructing an FSE index from coordinate vectors.
///
/// # Runtime Role
///
/// `FSEBuilder` owns the construction configuration and recursively creates a
/// hierarchy of partition nodes.
///
/// # Formal Reference
///
/// This implements the construction pipeline:
///
/// 1. Compute local centroid.
/// 2. Compute bounded support.
/// 3. Encode residuals.
/// 4. Select a geometrically useful split.
/// 5. Recurse only when subdivision improves structural tightness, unless the
///    partition exceeds the hard leaf cardinality limit.
#[derive(Clone, Debug, PartialEq)]
pub struct FSEBuilder {
    config: BuildConfig,
}

impl FSEBuilder {
    /// Creates a builder from a build configuration.
    pub fn new(config: BuildConfig) -> Self {
        Self { config }
    }

    /// Returns the builder configuration.
    pub fn config(&self) -> &BuildConfig {
        &self.config
    }

    /// Builds an index from an encoded record batch.
    ///
    /// # Runtime Role
    ///
    /// This method is the typed-data entry point after semantic encoding has
    /// produced coordinate vectors.
    ///
    /// # Panics
    ///
    /// Panics when the encoded batch contains no vectors.
    pub fn build_encoded_batch(&self, batch: &EncodedRecordBatch) -> FSEIndex {
        self.try_build_encoded_batch(batch)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Builds an index from an encoded record batch and returns an error when invalid.
    pub fn try_build_encoded_batch(
        &self,
        batch: &EncodedRecordBatch,
    ) -> Result<FSEIndex, BuildInputError> {
        self.try_build(batch.vectors())
    }

    /// Builds an index from raw coordinate vectors.
    ///
    /// # Panics
    ///
    /// Panics when the point set is empty or dimensionality is inconsistent.
    pub fn build(&self, points: &[Vector]) -> FSEIndex {
        self.try_build(points)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Builds an index and returns an error when input points are invalid.
    ///
    /// # Runtime Role
    ///
    /// This constructor is intended for caller-facing input validation. It
    /// enforces the same input invariants as [`FSEBuilder::build`] without
    /// panicking.
    pub fn try_build(&self, points: &[Vector]) -> Result<FSEIndex, BuildInputError> {
        validate_build_points(points)?;
        let mut nodes = Vec::new();
        let root = self.build_node(points.to_vec(), 0, &mut nodes);

        Ok(FSEIndex::new(nodes, root))
    }

    /// Builds an index and validates the constructed result.
    ///
    /// # Runtime Role
    ///
    /// This is a report-only construction path. It always returns the
    /// constructed index with its validation report, even when validation fails.
    /// Use [`FSEBuilder::build_checked`] when invalid output should be rejected.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`FSEBuilder::build`].
    pub fn build_validated(&self, points: &[Vector]) -> ValidatedFSEIndex {
        self.try_build_validated(points)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Builds an index with validation and returns an error when input is invalid.
    ///
    /// # Runtime Role
    ///
    /// This is the checked counterpart to [`FSEBuilder::build_validated`].
    pub fn try_build_validated(
        &self,
        points: &[Vector],
    ) -> Result<ValidatedFSEIndex, BuildInputError> {
        let index = self.try_build(points)?;
        let validation = validate_index(&index, self.config.max_leaf_size);

        Ok(ValidatedFSEIndex { index, validation })
    }

    /// Builds an index and returns an error when validation fails.
    ///
    /// # Runtime Role
    ///
    /// This is the strict construction path for callers that need a validated
    /// hierarchy before query execution or benchmark reporting. Invalid builds
    /// return compact validation results and detailed diagnostics.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`FSEBuilder::build`].
    pub fn build_checked(
        &self,
        points: &[Vector],
    ) -> Result<ValidatedFSEIndex, BuildValidationError> {
        self.try_build_checked(points).map_err(|error| match error {
            BuildCheckedError::Input(input) => panic!("{input}"),
            BuildCheckedError::Validation(validation) => validation,
        })
    }

    /// Builds an index and returns an error when input or output validation fails.
    ///
    /// # Runtime Role
    ///
    /// This is the fully checked build path. Invalid input points return
    /// [`BuildInputError`]. Invalid constructed hierarchy output returns
    /// [`BuildValidationError`].
    pub fn try_build_checked(
        &self,
        points: &[Vector],
    ) -> Result<ValidatedFSEIndex, BuildCheckedError> {
        let validated = self.try_build_validated(points)?;

        if validated.validation.is_valid() {
            return Ok(validated);
        }

        let diagnostics = index_validation_diagnostics(&validated.index, self.config.max_leaf_size);

        Err(BuildValidationError::new(validated, diagnostics).into())
    }

    fn build_node(
        &self,
        points: Vec<Vector>,
        depth: usize,
        nodes: &mut Vec<PartitionNode>,
    ) -> usize {
        let id = nodes.len();

        if self.should_stop_without_split(points.len(), depth) {
            let node = PartitionNode::from_points(id, &points);
            nodes.push(node);
            return id;
        }

        let Some(split) = self.accepted_structural_split(&points) else {
            // optional split didnt earn the extra node
            let node = PartitionNode::from_points(id, &points);
            nodes.push(node);
            return id;
        };

        if self.config.require_positive_split_volume_reduction && !split.was_forced {
            debug_assert!(
                accepts_split_quality(&split.metrics),
                "optional accepted split should improve bounding volume or degenerate extent"
            );
        }

        let placeholder = PartitionNode::internal_from_points(id, &points, Vec::new());
        nodes.push(placeholder);

        let left_id = self.build_node(split.left_points, depth + 1, nodes);
        let right_id = self.build_node(split.right_points, depth + 1, nodes);

        nodes[id].children = vec![left_id, right_id];

        id
    }

    fn should_stop_without_split(&self, point_count: usize, depth: usize) -> bool {
        point_count <= self.config.target_leaf_size || depth >= self.config.max_depth
    }

    fn should_force_split(&self, point_count: usize) -> bool {
        point_count > self.config.max_leaf_size
    }

    fn accepted_structural_split(&self, points: &[Vector]) -> Option<AcceptedStructuralSplit> {
        let split = best_structural_split(points);
        let was_forced = self.should_force_split(points.len());
        let metrics = split.score.metrics;

        if self.config.require_positive_split_volume_reduction
            && !was_forced
            && !accepts_split_quality(&metrics)
        {
            // geometry gate only applies while we are still under the hard cap
            return None;
        }

        Some(AcceptedStructuralSplit {
            left_points: split.left_points,
            right_points: split.right_points,
            metrics,
            was_forced,
        })
    }
}

fn validate_build_points(points: &[Vector]) -> Result<(), BuildInputError> {
    if points.is_empty() {
        return Err(BuildInputError::EmptyPointSet);
    }

    try_compute_centroid(points)?;

    Ok(())
}
