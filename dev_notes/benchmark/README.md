# Benchmark Notes

This directory contains the split benchmark documentation for the FSE Rust implementation.

The original benchmark notes entry point remains:

    dev_notes/benchmark_acceptance.md

That file now acts as an index into these topic files.

## Files

| File | Purpose |
| --- | --- |
| `acceptance-rules.md` | Core benchmark acceptance thresholds, revert rules, and performance decision rules. |
| `datasets-and-targets.md` | Small and large dataset baselines, max-depth choices, and target workload names. |
| `diagnostic-conclusions.md` | Boundary fixed-cost conclusions, retained execution conclusions, and result ownership conclusions. |
| `count-only-workflow.md` | Count-only API timing, count-only summary artifacts, and count-only comparison artifacts. |
| `existence-workflow.md` | Exact existence API timing, debug evidence scope, and artifact promotion rules. |
| `reference-visitor-workflow.md` | Reference visitor output contract, materialization evidence, and interpretation limits. |
| `review-workflow.md` | Low-gap review runner, target workload review, trial comparison commands, and expected review flow. |
| `validation-coverage.md` | Review validator artifact contracts, equivalence gates, comparison scope, and failure conditions. |
| `artifact-workflow.md` | Artifact organization, organized run folders, organized previous-label lookup, organized copy, and cleanup. |
| `leaf-policy.md` | Leaf policy experiments and the current 8/8 policy conclusion. |

## Current benchmark posture

The benchmark workflow currently separates:

    owned-result query execution
    reference-result query execution
    reference-visitor query execution
    row-view query execution
    count-only query execution
    existence query execution
    index footprint reporting
    debug-only retained execution diagnostics

Owned-result timing answers materialized row-return performance.

Reference-result timing answers exact row-reference collection performance.

Reference visitor timing answers exact row-reference delivery without building the reference result Vec.

Row-view timing answers borrowed exact row-view delivery without building owned row vectors.

Count-only timing answers exact-cardinality performance.

Existence timing answers exact non-empty-result performance and inspected-record behavior.

Index footprint reporting answers how many coordinate-like scalars are represented by residuals, centroids, bounds, and the total counted FSE index footprint.

Debug-only retained diagnostics explain implementation costs but should not be treated as public API benchmark evidence.

## Current benchmark phase

The project is currently in the benchmarking, diagnostic tooling, and measurement-discipline phase.

The core FSE implementation is already benchmarked against:

    flat scan
    KD-tree
    R-tree

The benchmark workflow now has:

    owned-result query execution
    count-only query execution
    existence query execution
    reference visitor materialization evidence for small and large datasets
    row-view materialization summary evidence
    repeated materialization mode trial summaries
    per-workload CSV metrics
    index footprint metrics
    baseline footprint metrics
    index footprint comparison metrics
    count-only workload summary artifacts
    count-only comparison artifacts
    target workload diagnostics
    small and large dataset review workflows
    review artifact validation gates
    organized artifact management
    benchmark history ledgers, including index footprint and baseline footprint history

## Primary review files

For acceptance and performance decisions, start with:

    acceptance-rules.md

For dataset configuration and target workload interpretation, use:

    datasets-and-targets.md

For why the current performance direction moved toward count-only/output-contract work, use:

    diagnostic-conclusions.md

For count-only query mode behavior and count-only artifact interpretation, use:

    count-only-workflow.md

For exact existence query behavior, inspected-record evidence, and artifact-scope decisions, use:

    existence-workflow.md

For reference visitor materialization evidence and interpretation limits, use:

    reference-visitor-workflow.md

For review commands and generated review artifacts, use:

    review-workflow.md

For organizing, copying, resolving, and cleaning benchmark artifacts, use:

    artifact-workflow.md

For review validator artifact contracts and equivalence gates, use:

    validation-coverage.md

For the full previous/current comparison validation path, use:

    review-workflow.md

For leaf policy decisions, use:

    leaf-policy.md

## Documentation validation

After editing the split benchmark notes, run:

    .\scripts\benchmark\check\check-benchmark-docs.ps1

The check validates the markdown files under:

    dev_notes/benchmark/

It catches malformed code fences and unclosed code fences before the notes are committed.

## Preferred review commands

Small dataset review:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "small" `
      -Trials 5 `
      -Iterations 10000

Large dataset review:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000

Large dataset review with organized artifact copy:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000 `
      -CopyArtifactsToRunFolder

Repeated materialization mode trials:

    .\scripts\benchmark\run\run-materialization-mode-trials.ps1 `
      -RunNumber 3 `
      -RunIndex 1 `
      -RunTopic "row view materialization trials" `
      -Dataset "small" `
      -Trials 5 `
      -Iterations 1000

## Current dataset defaults

Small dataset:

    max depth: 8
    target workload: cluster_boundary_range
    expected validation: pass

Large dataset:

    max depth: 16
    target workload: large_cross_cluster_boundary
    expected validation: pass

Do not compare large-depth-8 artifacts against large-depth-16 artifacts as equivalent performance baselines.

## Important benchmark rules

Use repeated-trial evidence, not one noisy benchmark run.

Use repeated materialization mode trials for output-contract timing decisions when single materialization summaries are too noisy.

Reporting/tooling-only commits should be judged by artifact correctness, not timing movement alone.

Performance commits should be accepted only when correctness is preserved and repeated-trial evidence supports the change.

Owned-result benchmark conclusions must stay separate from count-only benchmark conclusions.

Debug-only diagnostic timing should explain behavior, not replace public benchmark evidence.

## Important count-only rule

The most important count-only correctness field is:

    all_stats_match_owned

or, at row level:

    count_only_stats_match_owned

Count-only timing should be used as evidence only when the relevant stats match owned-result structural stats.

## Important artifact rule

The flat artifact root is useful for active comparison workflows.

The organized run folder is useful for long-term inspection and cleanup.

Flat root:

    benchmark_artifacts/

Organized run folder:

    benchmark_artifacts/runs/<label>/

Previous-label comparisons can resolve previous artifacts from either the flat root or organized run folders.

## Current interpretation

Current benchmark interpretation:

    FSE pruning is doing its job on the target workloads
    small boundary workload is near the fixed-cost floor under owned-result execution
    large target confirms result materialization dominates retained execution
    logical index footprint reporting is available for tracking structural metadata cost
    logical baseline footprint reporting is available for comparing baseline structural metadata cost
    count-only query mode is a valid separate output contract
    existence query mode is a valid separate output contract with timing-summary benchmark evidence
    reference visitor is a valid exact reference-delivery output contract with small and large materialization evidence
    row-view query mode is a valid borrowed row-delivery output contract but not the current performance direction
    repeated materialization trials are available for output-contract stability checks
    future traversal or boundary-specific optimizations need new evidence before being prioritized

Current row-view decision:

    repeated small and large materialization trials preserved exact agreement
    row-view did not beat reference-visitor on any matched large workload
    row-view should not be expanded into batch or reference-list APIs for performance without new evidence

Current footprint interpretation:

    footprint metrics are logical scalar counts, not byte-level memory measurements
    encoded coordinate scalar count is the current baseline for footprint comparison
    structural metadata scalars are counted separately from residual scalars
    index-footprint-history.csv is the cross-run ledger for these fields
    baseline-footprint-history.csv is the cross-run ledger for per-baseline footprint fields
    footprint reporting does not prove storage replacement by itself
