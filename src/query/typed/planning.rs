//! Planning diagnostics for typed query execution.

use crate::math::{BoundingBox, Scalar};

use super::append_delta::TypedAppendDeltaQueryView;
use super::index::TypedQueryIndex;
use super::plan::TypedQueryPlan;

const BROAD_QUERY_CANDIDATE_RATIO: Scalar = 0.80;
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

    /// Estimated base-index leaves retained by geometric traversal.
    pub estimated_retained_leaf_count: usize,

    /// Estimated retained leaves divided by total base-index leaves.
    pub estimated_retained_leaf_ratio: Scalar,

    /// Whether append-delta records must be directly evaluated.
    pub requires_append_delta_scan: bool,
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
        estimated_retained_leaf_count: base.estimated_retained_leaf_count,
        estimated_retained_leaf_ratio: base.estimated_retained_leaf_ratio,
        requires_append_delta_scan,
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
        };
    }

    let estimate = estimate_candidate_ratio(plan, &root.bounds, base_record_count);
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
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CandidateRatioEstimate {
    candidate_ratio: Scalar,
    constrained_dimensions: usize,
}

fn estimate_candidate_ratio(
    plan: &TypedQueryPlan,
    root_bounds: &BoundingBox,
    total_records: usize,
) -> CandidateRatioEstimate {
    let mut candidate_ratio = 1.0;
    let mut constrained_dimensions = 0;
    let minimum_intersecting_fraction = minimum_intersecting_fraction(total_records);

    for dimension in 0..plan.query_region().dimensions() {
        let fraction = dimension_candidate_fraction(
            plan.query_region().min[dimension],
            plan.query_region().max[dimension],
            root_bounds.min[dimension],
            root_bounds.max[dimension],
            minimum_intersecting_fraction,
        );

        if fraction < 1.0 {
            constrained_dimensions += 1;
        }

        candidate_ratio *= fraction;
    }

    CandidateRatioEstimate {
        candidate_ratio: candidate_ratio.clamp(0.0, 1.0),
        constrained_dimensions,
    }
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

fn ratio(part: usize, total: usize) -> Scalar {
    if total == 0 {
        return 0.0;
    }

    part as Scalar / total as Scalar
}
