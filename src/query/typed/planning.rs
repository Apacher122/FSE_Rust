//! Planning diagnostics for typed query execution.

use std::collections::HashMap;

use crate::data::{FSERecordBatch, FSEValue};
use crate::math::{BoundingBox, Scalar};

use super::append_delta::TypedAppendDeltaQueryView;
use super::index::TypedQueryIndex;
use super::plan::TypedQueryPlan;

const BROAD_QUERY_CANDIDATE_RATIO: Scalar = 0.75;
const SELECTIVE_QUERY_CANDIDATE_RATIO: Scalar = 0.35;
const HIGH_DIMENSION_COUNT: usize = 8;
const LOW_CONSTRAINED_DIMENSION_COUNT: usize = 1;

/// Caller-visible result shape for typed query planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedQueryOutputContract {
    /// Return matching logical row identifiers.
    RowIds,

    /// Return matching typed records.
    Rows,

    /// Return exact match cardinality.
    Count,

    /// Return whether at least one exact match exists.
    Existence,

    /// Visit matching logical row identifiers.
    RowIdVisitor,

    /// Visit matching typed records.
    RowVisitor,
}

impl TypedQueryOutputContract {
    /// Returns true when this contract returns or visits typed records.
    pub fn materializes_records(self) -> bool {
        matches!(self, Self::Rows | Self::RowVisitor)
    }

    /// Returns true when this contract may finish after the first exact match.
    pub fn short_circuits(self) -> bool {
        matches!(self, Self::Existence)
    }
}

/// Estimated work implied by a typed query planning decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypedQueryPlanningWorkEstimate {
    /// Estimated hierarchy nodes inspected during geometric traversal.
    pub estimated_traversal_node_visits: usize,

    /// Estimated records reconstructed from retained FSE partitions.
    pub estimated_reconstructed_records: usize,

    /// Estimated records evaluated against typed predicates.
    pub estimated_predicate_evaluations: usize,

    /// Estimated records delivered through a typed-record output contract.
    pub estimated_materialized_records: usize,

    /// Estimated records evaluated by direct batch scanning.
    pub estimated_flat_scan_records: usize,
}

impl TypedQueryPlanningWorkEstimate {
    /// Returns an estimate with no planned work.
    pub fn empty() -> Self {
        Self {
            estimated_traversal_node_visits: 0,
            estimated_reconstructed_records: 0,
            estimated_predicate_evaluations: 0,
            estimated_materialized_records: 0,
            estimated_flat_scan_records: 0,
        }
    }

    /// Returns true when every estimated work component is zero.
    pub fn is_empty(&self) -> bool {
        *self == Self::empty()
    }
}

/// Estimated work for each typed query execution strategy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypedQueryStrategyCostEstimates {
    /// Estimated work when no execution path is required.
    pub no_op: TypedQueryPlanningWorkEstimate,

    /// Estimated work for geometric traversal through the base FSE index.
    pub fse_traversal: TypedQueryPlanningWorkEstimate,

    /// Estimated work for direct typed predicate evaluation over visible records.
    pub flat_scan: TypedQueryPlanningWorkEstimate,

    /// Estimated work for base-index traversal plus append-delta evaluation.
    pub hybrid: TypedQueryPlanningWorkEstimate,
}

impl TypedQueryStrategyCostEstimates {
    /// Returns the estimate for an execution strategy.
    pub fn for_strategy(
        &self,
        strategy: TypedQueryExecutionStrategy,
    ) -> TypedQueryPlanningWorkEstimate {
        match strategy {
            TypedQueryExecutionStrategy::NoOp => self.no_op,
            TypedQueryExecutionStrategy::FseTraversal => self.fse_traversal,
            TypedQueryExecutionStrategy::FlatScan => self.flat_scan,
            TypedQueryExecutionStrategy::Hybrid => self.hybrid,
        }
    }
}

/// Work deltas between the selected typed strategy and direct flat scanning.
///
/// Delta fields are computed as selected work minus direct flat-scan work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypedQueryPlanningCostComparison {
    /// Execution strategy selected by the planner.
    pub selected_strategy: TypedQueryExecutionStrategy,

    /// Estimated work for the selected strategy.
    pub selected_work: TypedQueryPlanningWorkEstimate,

    /// Estimated work for direct flat scanning.
    pub flat_scan_work: TypedQueryPlanningWorkEstimate,

    /// Delta for hierarchy node visits.
    pub traversal_node_visit_delta: i128,

    /// Delta for reconstructed records.
    pub reconstructed_record_delta: i128,

    /// Delta for typed predicate evaluations.
    pub predicate_evaluation_delta: i128,

    /// Delta for materialized typed records.
    pub materialized_record_delta: i128,

    /// Delta for directly scanned batch records.
    pub flat_scan_record_delta: i128,
}

impl TypedQueryPlanningCostComparison {
    fn new(
        selected_strategy: TypedQueryExecutionStrategy,
        selected_work: TypedQueryPlanningWorkEstimate,
        flat_scan_work: TypedQueryPlanningWorkEstimate,
    ) -> Self {
        Self {
            selected_strategy,
            selected_work,
            flat_scan_work,
            traversal_node_visit_delta: work_delta(
                selected_work.estimated_traversal_node_visits,
                flat_scan_work.estimated_traversal_node_visits,
            ),
            reconstructed_record_delta: work_delta(
                selected_work.estimated_reconstructed_records,
                flat_scan_work.estimated_reconstructed_records,
            ),
            predicate_evaluation_delta: work_delta(
                selected_work.estimated_predicate_evaluations,
                flat_scan_work.estimated_predicate_evaluations,
            ),
            materialized_record_delta: work_delta(
                selected_work.estimated_materialized_records,
                flat_scan_work.estimated_materialized_records,
            ),
            flat_scan_record_delta: work_delta(
                selected_work.estimated_flat_scan_records,
                flat_scan_work.estimated_flat_scan_records,
            ),
        }
    }

    /// Returns true when selected predicate evaluation work is lower than flat scan.
    pub fn reduces_predicate_evaluations(self) -> bool {
        self.predicate_evaluation_delta < 0
    }

    /// Returns true when selected direct batch scanning work is lower than flat scan.
    pub fn reduces_flat_scan_records(self) -> bool {
        self.flat_scan_record_delta < 0
    }

    /// Classifies the selected strategy relative to direct flat scanning.
    pub fn classification(self) -> TypedQueryPlanningCostClassification {
        if self.selected_work.is_empty() {
            return TypedQueryPlanningCostClassification::NoWork;
        }

        if self.reduces_predicate_evaluations() || self.reduces_flat_scan_records() {
            return TypedQueryPlanningCostClassification::ScanWorkReduction;
        }

        if self.has_no_delta() {
            return TypedQueryPlanningCostClassification::FlatScanEquivalent;
        }

        TypedQueryPlanningCostClassification::TraversalOverheadWithoutScanReduction
    }

    fn has_no_delta(self) -> bool {
        self.traversal_node_visit_delta == 0
            && self.reconstructed_record_delta == 0
            && self.predicate_evaluation_delta == 0
            && self.materialized_record_delta == 0
            && self.flat_scan_record_delta == 0
    }
}

/// Classification of selected planner work relative to direct flat scanning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedQueryPlanningCostClassification {
    /// The selected strategy has no estimated work.
    NoWork,

    /// The selected strategy reduces predicate evaluation or direct scan work.
    ScanWorkReduction,

    /// The selected strategy has the same estimated work shape as flat scan.
    FlatScanEquivalent,

    /// The selected strategy adds traversal work without reducing scan work.
    TraversalOverheadWithoutScanReduction,
}

/// Estimated selectivity class for a typed query plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedQuerySelectivityBucket {
    /// The query is estimated to match no records.
    Empty,

    /// The query is estimated to retain a small fraction of visible records.
    Selective,

    /// The query is estimated to retain a middle fraction of visible records.
    Moderate,

    /// The query is estimated to retain most visible records.
    Broad,
}

impl TypedQuerySelectivityBucket {
    /// Classifies an estimated candidate ratio.
    pub fn from_candidate_ratio(candidate_ratio: Scalar) -> Self {
        if candidate_ratio <= 0.0 {
            Self::Empty
        } else if candidate_ratio <= SELECTIVE_QUERY_CANDIDATE_RATIO {
            Self::Selective
        } else if candidate_ratio >= BROAD_QUERY_CANDIDATE_RATIO {
            Self::Broad
        } else {
            Self::Moderate
        }
    }

    /// Returns true for broad estimated result sets.
    pub fn is_broad(self) -> bool {
        matches!(self, Self::Broad)
    }
}

/// Risk flags surfaced by typed query planning diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypedQueryPlanningRiskFlags {
    /// The estimated candidate set is broad relative to visible records.
    pub broad_predicate: bool,

    /// The output contract materializes records for a broad estimated result.
    pub materialization_pressure: bool,

    /// A high-dimensional query constrains too few dimensions for stable pruning.
    pub high_dimensional_low_constraint: bool,

    /// Pending append-delta records require direct predicate evaluation.
    pub append_delta_scan: bool,
}

impl TypedQueryPlanningRiskFlags {
    /// Returns true when any planning risk flag is set.
    pub fn has_any(self) -> bool {
        self.broad_predicate
            || self.materialization_pressure
            || self.high_dimensional_low_constraint
            || self.append_delta_scan
    }
}

/// Distribution metadata used by typed query planning estimates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TypedQueryPlanningStatistics {
    total_records: usize,
    categorical_counts: HashMap<TypedCategoricalStatisticsKey, usize>,
}

impl TypedQueryPlanningStatistics {
    /// Derives planning statistics from a typed record batch.
    pub(super) fn from_batch(batch: &FSERecordBatch) -> Self {
        let mut categorical_counts = HashMap::new();

        for record in batch.records() {
            for (field, value) in record.values().iter().enumerate() {
                let FSEValue::Category(category) = value else {
                    continue;
                };

                *categorical_counts
                    .entry(TypedCategoricalStatisticsKey::new(field, category.clone()))
                    .or_insert(0) += 1;
            }
        }

        Self {
            total_records: batch.len(),
            categorical_counts,
        }
    }

    fn categorical_equality_ratio(&self, field: usize, category: &str) -> Option<Scalar> {
        if self.total_records == 0 {
            return Some(0.0);
        }

        let count = self
            .categorical_counts
            .get(&TypedCategoricalStatisticsKey::new(
                field,
                category.to_string(),
            ))
            .copied()
            .unwrap_or(0);

        Some(ratio(count, self.total_records))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TypedCategoricalStatisticsKey {
    field: usize,
    category: String,
}

impl TypedCategoricalStatisticsKey {
    fn new(field: usize, category: String) -> Self {
        Self { field, category }
    }
}

/// Execution strategy selected by typed query planning diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedQueryExecutionStrategy {
    /// No scan is needed for the supplied plan and data shape.
    NoOp,

    /// Use the FSE hierarchy for geometric traversal.
    FseTraversal,

    /// Evaluate typed predicates directly against the record batch.
    FlatScan,

    /// Use FSE traversal for indexed records and direct evaluation for append deltas.
    Hybrid,
}

/// Primary explanation for a typed query planning decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedQueryPlanningReason {
    /// The typed query plan has no satisfying coordinate region.
    UnsatisfiablePlan,

    /// The indexed dataset contains no records.
    EmptyIndex,

    /// The query region has different dimensionality than the indexed geometry.
    DimensionMismatch,

    /// The query region has no estimated overlap with indexed root bounds.
    NoCandidatesEstimated,

    /// Pending append-delta records require direct typed evaluation.
    AppendDeltaScan,

    /// The query region is selective enough to favor FSE traversal.
    SelectiveGeometry,

    /// The query region has moderate estimated candidate pressure.
    ModerateGeometry,

    /// The query region is broad and the output contract returns typed records.
    BroadMaterializedQuery,

    /// The query region is broad relative to indexed root bounds.
    BroadGeometry,

    /// A broad categorical equality predicate was estimated from observed category frequency.
    BroadCategoricalEquality,

    /// A high-dimensional query constrains too few dimensions for a stable pruning estimate.
    HighDimensionalLowConstraintQuery,
}

/// Diagnostic report produced before typed query execution.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedQueryPlanningDiagnostics {
    /// Selected execution strategy.
    pub strategy: TypedQueryExecutionStrategy,

    /// Primary reason for the selected strategy.
    pub reason: TypedQueryPlanningReason,

    /// Requested output contract.
    pub output_contract: TypedQueryOutputContract,

    /// Total records visible to the query.
    pub total_records: usize,

    /// Records stored in the base typed query index.
    pub base_record_count: usize,

    /// Records stored in the append-delta batch.
    pub append_delta_record_count: usize,

    /// Number of FSE hierarchy nodes in the base index.
    pub node_count: usize,

    /// Number of leaf partitions in the base index.
    pub leaf_count: usize,

    /// Dimensionality of the base index geometry.
    pub dimensions: usize,

    /// Number of validated typed predicates in the plan.
    pub predicate_count: usize,

    /// Number of dimensions constrained by the query region estimate.
    pub constrained_dimensions: usize,

    /// Estimated records that require typed predicate evaluation.
    pub estimated_candidate_records: usize,

    /// Estimated candidate records divided by total visible records.
    pub estimated_candidate_ratio: Scalar,

    /// Selectivity bucket derived from the estimated candidate ratio.
    pub selectivity_bucket: TypedQuerySelectivityBucket,

    /// Estimated base-index leaves retained by geometric traversal.
    pub estimated_retained_leaf_count: usize,

    /// Estimated retained leaves divided by total base-index leaves.
    pub estimated_retained_leaf_ratio: Scalar,

    /// Whether category-frequency statistics contributed to the candidate estimate.
    pub uses_categorical_frequency_estimate: bool,

    /// Whether append-delta records must be directly evaluated.
    pub requires_append_delta_scan: bool,

    /// Estimated work implied by the selected execution strategy.
    pub work_estimate: TypedQueryPlanningWorkEstimate,

    /// Estimated work for each candidate execution strategy.
    pub strategy_costs: TypedQueryStrategyCostEstimates,

    /// Risk flags derived from the planning estimate and output contract.
    pub risk_flags: TypedQueryPlanningRiskFlags,
}

impl TypedQueryPlanningDiagnostics {
    /// Returns true when the selected strategy uses base-index geometry.
    pub fn uses_geometric_traversal(&self) -> bool {
        matches!(
            self.strategy,
            TypedQueryExecutionStrategy::FseTraversal | TypedQueryExecutionStrategy::Hybrid
        )
    }

    /// Returns true when the selected strategy uses direct batch evaluation.
    pub fn uses_flat_scan(&self) -> bool {
        matches!(self.strategy, TypedQueryExecutionStrategy::FlatScan)
    }

    /// Returns true when the candidate estimate is broad.
    pub fn is_broad_query(&self) -> bool {
        self.selectivity_bucket.is_broad()
    }

    /// Compares the selected strategy with direct flat scanning.
    pub fn cost_comparison_against_flat_scan(&self) -> TypedQueryPlanningCostComparison {
        TypedQueryPlanningCostComparison::new(
            self.strategy,
            self.work_estimate,
            self.strategy_costs.flat_scan,
        )
    }

    /// Classifies selected work relative to direct flat scanning.
    pub fn cost_classification_against_flat_scan(&self) -> TypedQueryPlanningCostClassification {
        self.cost_comparison_against_flat_scan().classification()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BasePlanningEstimate {
    base_record_count: usize,
    node_count: usize,
    leaf_count: usize,
    dimensions: usize,
    query_dimensions: usize,
    constrained_dimensions: usize,
    estimated_candidate_records: usize,
    estimated_candidate_ratio: Scalar,
    estimated_retained_leaf_count: usize,
    estimated_retained_leaf_ratio: Scalar,
    uses_categorical_frequency_estimate: bool,
}

/// Produces planning diagnostics for a typed query index.
pub fn plan_typed_query_execution(
    index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    output_contract: TypedQueryOutputContract,
) -> TypedQueryPlanningDiagnostics {
    let base = estimate_base_index(index, plan);

    planning_diagnostics_from_estimate(base, plan, output_contract, 0)
}

/// Produces planning diagnostics for a typed query over pending append deltas.
pub fn plan_typed_append_delta_query_execution(
    view: &TypedAppendDeltaQueryView<'_>,
    plan: &TypedQueryPlan,
    output_contract: TypedQueryOutputContract,
) -> TypedQueryPlanningDiagnostics {
    let base = estimate_base_index(view.base(), plan);

    planning_diagnostics_from_estimate(base, plan, output_contract, view.appended().len())
}

fn planning_diagnostics_from_estimate(
    base: BasePlanningEstimate,
    plan: &TypedQueryPlan,
    output_contract: TypedQueryOutputContract,
    append_delta_record_count: usize,
) -> TypedQueryPlanningDiagnostics {
    let append_delta_candidate_records =
        append_delta_candidate_count(plan, append_delta_record_count);
    let total_records = base.base_record_count + append_delta_record_count;
    let estimated_candidate_records =
        base.estimated_candidate_records + append_delta_candidate_records;
    let estimated_candidate_ratio = ratio(estimated_candidate_records, total_records);
    let selectivity_bucket =
        TypedQuerySelectivityBucket::from_candidate_ratio(estimated_candidate_ratio);
    let requires_append_delta_scan = append_delta_candidate_records > 0;

    let (strategy, reason) = select_strategy(
        plan,
        output_contract,
        total_records,
        base,
        estimated_candidate_records,
        estimated_candidate_ratio,
        requires_append_delta_scan,
    );
    let strategy_costs = estimate_strategy_costs(
        output_contract,
        base,
        estimated_candidate_records,
        append_delta_record_count,
        total_records,
    );
    let work_estimate = strategy_costs.for_strategy(strategy);
    let risk_flags = risk_flags_from_estimate(
        reason,
        output_contract,
        selectivity_bucket,
        requires_append_delta_scan,
    );

    TypedQueryPlanningDiagnostics {
        strategy,
        reason,
        output_contract,
        total_records,
        base_record_count: base.base_record_count,
        append_delta_record_count,
        node_count: base.node_count,
        leaf_count: base.leaf_count,
        dimensions: base.dimensions,
        predicate_count: plan.predicates().len(),
        constrained_dimensions: base.constrained_dimensions,
        estimated_candidate_records,
        estimated_candidate_ratio,
        selectivity_bucket,
        estimated_retained_leaf_count: base.estimated_retained_leaf_count,
        estimated_retained_leaf_ratio: base.estimated_retained_leaf_ratio,
        uses_categorical_frequency_estimate: base.uses_categorical_frequency_estimate,
        requires_append_delta_scan,
        work_estimate,
        strategy_costs,
        risk_flags,
    }
}

fn select_strategy(
    plan: &TypedQueryPlan,
    output_contract: TypedQueryOutputContract,
    total_records: usize,
    base: BasePlanningEstimate,
    estimated_candidate_records: usize,
    estimated_candidate_ratio: Scalar,
    requires_append_delta_scan: bool,
) -> (TypedQueryExecutionStrategy, TypedQueryPlanningReason) {
    if plan.is_unsatisfiable() {
        return (
            TypedQueryExecutionStrategy::NoOp,
            TypedQueryPlanningReason::UnsatisfiablePlan,
        );
    }

    if total_records == 0 {
        return (
            TypedQueryExecutionStrategy::NoOp,
            TypedQueryPlanningReason::EmptyIndex,
        );
    }

    if base.dimensions != base.query_dimensions {
        return (
            TypedQueryExecutionStrategy::FlatScan,
            TypedQueryPlanningReason::DimensionMismatch,
        );
    }

    if estimated_candidate_records == 0 {
        return (
            TypedQueryExecutionStrategy::NoOp,
            TypedQueryPlanningReason::NoCandidatesEstimated,
        );
    }

    if requires_append_delta_scan {
        return (
            TypedQueryExecutionStrategy::Hybrid,
            TypedQueryPlanningReason::AppendDeltaScan,
        );
    }

    if estimated_candidate_ratio >= BROAD_QUERY_CANDIDATE_RATIO
        && base.uses_categorical_frequency_estimate
        && !output_contract.materializes_records()
    {
        return (
            TypedQueryExecutionStrategy::FlatScan,
            TypedQueryPlanningReason::BroadCategoricalEquality,
        );
    }

    if base.dimensions >= HIGH_DIMENSION_COUNT
        && base.constrained_dimensions <= LOW_CONSTRAINED_DIMENSION_COUNT
        && estimated_candidate_ratio >= SELECTIVE_QUERY_CANDIDATE_RATIO
    {
        return (
            TypedQueryExecutionStrategy::FlatScan,
            TypedQueryPlanningReason::HighDimensionalLowConstraintQuery,
        );
    }

    if estimated_candidate_ratio >= BROAD_QUERY_CANDIDATE_RATIO {
        if output_contract.materializes_records() {
            return (
                TypedQueryExecutionStrategy::FlatScan,
                TypedQueryPlanningReason::BroadMaterializedQuery,
            );
        }

        return (
            TypedQueryExecutionStrategy::FlatScan,
            TypedQueryPlanningReason::BroadGeometry,
        );
    }

    if estimated_candidate_ratio <= SELECTIVE_QUERY_CANDIDATE_RATIO && base.leaf_count > 1 {
        return (
            TypedQueryExecutionStrategy::FseTraversal,
            TypedQueryPlanningReason::SelectiveGeometry,
        );
    }

    (
        TypedQueryExecutionStrategy::FseTraversal,
        TypedQueryPlanningReason::ModerateGeometry,
    )
}

fn estimate_base_index(index: &TypedQueryIndex, plan: &TypedQueryPlan) -> BasePlanningEstimate {
    let fse_index = index.index().index();
    let root = fse_index.root_node();
    let base_record_count = index.batch().len();
    let dimensions = fse_index.dimensions;
    let query_dimensions = plan.query_region().dimensions();

    if plan.is_unsatisfiable() || dimensions != query_dimensions {
        return BasePlanningEstimate {
            base_record_count,
            node_count: fse_index.node_count(),
            leaf_count: fse_index.leaf_count(),
            dimensions,
            query_dimensions,
            constrained_dimensions: 0,
            estimated_candidate_records: 0,
            estimated_candidate_ratio: 0.0,
            estimated_retained_leaf_count: 0,
            estimated_retained_leaf_ratio: 0.0,
            uses_categorical_frequency_estimate: false,
        };
    }

    let estimate = estimate_candidate_ratio(
        plan,
        &root.bounds,
        base_record_count,
        index.planning_statistics(),
    );
    let estimated_candidate_records =
        estimated_count_from_ratio(estimate.candidate_ratio, base_record_count);
    let estimated_retained_leaf_count =
        estimated_count_from_ratio(estimate.candidate_ratio, fse_index.leaf_count());
    let estimated_retained_leaf_ratio =
        ratio(estimated_retained_leaf_count, fse_index.leaf_count());

    BasePlanningEstimate {
        base_record_count,
        node_count: fse_index.node_count(),
        leaf_count: fse_index.leaf_count(),
        dimensions,
        query_dimensions,
        constrained_dimensions: estimate.constrained_dimensions,
        estimated_candidate_records,
        estimated_candidate_ratio: estimate.candidate_ratio,
        estimated_retained_leaf_count,
        estimated_retained_leaf_ratio,
        uses_categorical_frequency_estimate: estimate.uses_categorical_frequency_estimate,
    }
}

fn estimate_strategy_costs(
    output_contract: TypedQueryOutputContract,
    base: BasePlanningEstimate,
    estimated_candidate_records: usize,
    append_delta_record_count: usize,
    total_records: usize,
) -> TypedQueryStrategyCostEstimates {
    TypedQueryStrategyCostEstimates {
        no_op: estimate_strategy_work(
            TypedQueryExecutionStrategy::NoOp,
            output_contract,
            base,
            estimated_candidate_records,
            append_delta_record_count,
            total_records,
        ),
        fse_traversal: estimate_strategy_work(
            TypedQueryExecutionStrategy::FseTraversal,
            output_contract,
            base,
            estimated_candidate_records,
            append_delta_record_count,
            total_records,
        ),
        flat_scan: estimate_strategy_work(
            TypedQueryExecutionStrategy::FlatScan,
            output_contract,
            base,
            estimated_candidate_records,
            append_delta_record_count,
            total_records,
        ),
        hybrid: estimate_strategy_work(
            TypedQueryExecutionStrategy::Hybrid,
            output_contract,
            base,
            estimated_candidate_records,
            append_delta_record_count,
            total_records,
        ),
    }
}

fn estimate_strategy_work(
    strategy: TypedQueryExecutionStrategy,
    output_contract: TypedQueryOutputContract,
    base: BasePlanningEstimate,
    estimated_candidate_records: usize,
    append_delta_record_count: usize,
    total_records: usize,
) -> TypedQueryPlanningWorkEstimate {
    match strategy {
        TypedQueryExecutionStrategy::NoOp => TypedQueryPlanningWorkEstimate::empty(),
        TypedQueryExecutionStrategy::FlatScan => TypedQueryPlanningWorkEstimate {
            estimated_traversal_node_visits: 0,
            estimated_reconstructed_records: 0,
            estimated_predicate_evaluations: total_records,
            estimated_materialized_records: estimated_materialized_records(
                output_contract,
                estimated_candidate_records,
            ),
            estimated_flat_scan_records: total_records,
        },
        TypedQueryExecutionStrategy::FseTraversal => TypedQueryPlanningWorkEstimate {
            estimated_traversal_node_visits: estimated_traversal_node_visits(base),
            estimated_reconstructed_records: base.estimated_candidate_records,
            estimated_predicate_evaluations: base.estimated_candidate_records,
            estimated_materialized_records: estimated_materialized_records(
                output_contract,
                base.estimated_candidate_records,
            ),
            estimated_flat_scan_records: 0,
        },
        TypedQueryExecutionStrategy::Hybrid => TypedQueryPlanningWorkEstimate {
            estimated_traversal_node_visits: estimated_traversal_node_visits(base),
            estimated_reconstructed_records: base.estimated_candidate_records,
            estimated_predicate_evaluations: base
                .estimated_candidate_records
                .saturating_add(append_delta_record_count),
            estimated_materialized_records: estimated_materialized_records(
                output_contract,
                estimated_candidate_records,
            ),
            estimated_flat_scan_records: append_delta_record_count,
        },
    }
}

fn estimated_traversal_node_visits(base: BasePlanningEstimate) -> usize {
    if base.node_count == 0 || base.estimated_retained_leaf_count == 0 {
        return 0;
    }

    let internal_node_count = base.node_count.saturating_sub(base.leaf_count);
    let retained_internal_count =
        estimated_count_from_ratio(base.estimated_retained_leaf_ratio, internal_node_count);

    base.estimated_retained_leaf_count
        .saturating_add(retained_internal_count)
        .clamp(1, base.node_count)
}

fn estimated_materialized_records(
    output_contract: TypedQueryOutputContract,
    estimated_candidate_records: usize,
) -> usize {
    if output_contract.materializes_records() {
        estimated_candidate_records
    } else {
        0
    }
}

fn risk_flags_from_estimate(
    reason: TypedQueryPlanningReason,
    output_contract: TypedQueryOutputContract,
    selectivity_bucket: TypedQuerySelectivityBucket,
    requires_append_delta_scan: bool,
) -> TypedQueryPlanningRiskFlags {
    let broad_predicate = selectivity_bucket.is_broad();

    TypedQueryPlanningRiskFlags {
        broad_predicate,
        materialization_pressure: broad_predicate && output_contract.materializes_records(),
        high_dimensional_low_constraint: matches!(
            reason,
            TypedQueryPlanningReason::HighDimensionalLowConstraintQuery
        ),
        append_delta_scan: requires_append_delta_scan,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CandidateRatioEstimate {
    candidate_ratio: Scalar,
    constrained_dimensions: usize,
    uses_categorical_frequency_estimate: bool,
}

fn estimate_candidate_ratio(
    plan: &TypedQueryPlan,
    root_bounds: &BoundingBox,
    total_records: usize,
    statistics: &TypedQueryPlanningStatistics,
) -> CandidateRatioEstimate {
    let mut candidate_ratio = 1.0;
    let mut constrained_dimensions = 0;
    let mut uses_categorical_frequency_estimate = false;
    let minimum_intersecting_fraction = minimum_intersecting_fraction(total_records);

    for dimension in 0..plan.query_region().dimensions() {
        let categorical_fraction =
            categorical_equality_fraction_for_dimension(plan, statistics, dimension);
        let fraction = categorical_fraction.unwrap_or_else(|| {
            dimension_candidate_fraction(
                plan.query_region().min[dimension],
                plan.query_region().max[dimension],
                root_bounds.min[dimension],
                root_bounds.max[dimension],
                minimum_intersecting_fraction,
            )
        });

        if categorical_fraction.is_some() {
            uses_categorical_frequency_estimate = true;
        }

        if fraction < 1.0 {
            constrained_dimensions += 1;
        }

        candidate_ratio *= fraction;
    }

    CandidateRatioEstimate {
        candidate_ratio: candidate_ratio.clamp(0.0, 1.0),
        constrained_dimensions,
        uses_categorical_frequency_estimate,
    }
}

fn categorical_equality_fraction_for_dimension(
    plan: &TypedQueryPlan,
    statistics: &TypedQueryPlanningStatistics,
    dimension: usize,
) -> Option<Scalar> {
    if plan.predicates().len() != 1 {
        return None;
    }

    plan.categorical_equality_dimensions()
        .iter()
        .filter(|constraint| constraint.dimension() == dimension)
        .filter_map(|constraint| {
            statistics.categorical_equality_ratio(constraint.field(), constraint.category())
        })
        .reduce(Scalar::min)
}

fn dimension_candidate_fraction(
    query_min: Scalar,
    query_max: Scalar,
    root_min: Scalar,
    root_max: Scalar,
    minimum_intersecting_fraction: Scalar,
) -> Scalar {
    if query_max < root_min || query_min > root_max {
        return 0.0;
    }

    let root_span = root_max - root_min;

    if root_span <= 0.0 {
        return if query_min <= root_min && query_max >= root_max {
            1.0
        } else {
            0.0
        };
    }

    let clipped_min = query_min.max(root_min);
    let clipped_max = query_max.min(root_max);
    let clipped_span = clipped_max - clipped_min;

    if clipped_span <= 0.0 {
        return minimum_intersecting_fraction;
    }

    (clipped_span / root_span)
        .max(minimum_intersecting_fraction)
        .clamp(0.0, 1.0)
}

fn append_delta_candidate_count(plan: &TypedQueryPlan, append_delta_record_count: usize) -> usize {
    if plan.is_unsatisfiable() {
        return 0;
    }

    append_delta_record_count
}

fn minimum_intersecting_fraction(total_records: usize) -> Scalar {
    if total_records == 0 {
        return 0.0;
    }

    1.0 / total_records as Scalar
}

fn estimated_count_from_ratio(candidate_ratio: Scalar, total: usize) -> usize {
    if total == 0 || candidate_ratio <= 0.0 {
        return 0;
    }

    ((candidate_ratio * total as Scalar).ceil() as usize).clamp(1, total)
}

fn work_delta(selected: usize, baseline: usize) -> i128 {
    selected as i128 - baseline as i128
}

fn ratio(part: usize, total: usize) -> Scalar {
    if total == 0 {
        return 0.0;
    }

    part as Scalar / total as Scalar
}
