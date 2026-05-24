# Benchmark Review Workflow

This file documents the benchmark review workflow for the FSE Rust implementation.

The review workflow is centered on:

    scripts/run-low-gap-review.ps1

This script generates the normal benchmark artifact bundle, repeated low-gap trials, workload summaries, target workload summaries, count-only summaries, comparison artifacts, review notes, and a review manifest.

## Review purpose

The benchmark review workflow exists to keep performance decisions disciplined.

It should answer:

    did validation pass
    did owned-result FSE timing improve or regress
    did count-only timing improve or regress
    did target workload behavior change
    did structural counters change
    did artifact generation succeed
    are comparisons available or skipped
    are skipped comparisons explained

Use repeated-trial evidence for performance claims.

Do not rely on one noisy single-run benchmark result.

## Current review phase

The project is currently in the benchmarking, diagnostic tooling, and measurement-discipline phase.

The core FSE implementation is already benchmarked against:

    flat_scan
    kd_tree
    r_tree

The review workflow tracks two separate output contracts:

    owned-result query execution:
      returns materialized Vec<Vector> results

    count-only query execution:
      returns exact cardinality without owned Vector materialization

Owned-result conclusions and count-only conclusions must remain separate.

## Primary review command

Small dataset review:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "small" `
      -Trials 5 `
      -Iterations 10000

Large dataset review:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000

Large dataset review with organized artifact copy:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000 `
      -CopyArtifactsToRunFolder

If re-running the same label and intentionally replacing the organized copy:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000 `
      -CopyArtifactsToRunFolder `
      -ForceOrganizedArtifacts

## Dataset defaults

Small dataset default:

    dataset: small
    max depth: 8
    target workload: cluster_boundary_range
    expected validation: pass

Large dataset default:

    dataset: large
    max depth: 16
    target workload: large_cross_cluster_boundary
    expected validation: pass

The large dataset should use max depth 16 by default.

Do not use old large-depth-8 artifacts as performance baselines because that shape was invalid for max leaf size 8.

## Manual target workload override

The review runner chooses the target workload from the selected dataset.

Default target workloads:

    small dataset:
      cluster_boundary_range

    large dataset:
      large_cross_cluster_boundary

Override the target workload only when intentionally investigating a specific workload:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -Dataset "large" `
      -TargetWorkloadName "<target-workload-name>" `
      -Trials 5 `
      -Iterations 10000

Manual target summary command:

    .\scripts\summarize-target-workload-trials.ps1 `
      -Label "<label>" `
      -InputLabel "<label>" `
      -TargetWorkloadName "<target-workload-name>" `
      -OutputDir "benchmark_artifacts"

## Manual max-depth override

Manual max-depth override is allowed only when intentionally testing build-depth behavior or comparing alternate valid build shapes.

Example:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -Dataset "large" `
      -MaxDepth 16 `
      -Trials 5 `
      -Iterations 10000

Do not use max-depth overrides casually.

Max depth changes index shape, retained leaf counts, candidate counts, and timing.

If max depth changes, the review notes should explain:

    why max depth changed
    whether validation passed
    how index shape changed
    whether target workload counters changed
    whether old artifacts remain comparable

## Normal benchmark artifact bundle

The review runner first generates the normal benchmark artifact bundle.

The normal bundle includes:

    benchmark-output-<label>.txt
    debug-output-<label>.txt
    summary-<label>.csv
    workloads-<label>.csv
    count-only-workload-summary-<label>.csv
    count-only-workload-summary-<label>.txt
    low-selectivity-gap-<label>.csv
    low-gap-regression-notes-<label>.txt

The benchmark output should show:

    dataset
    record count
    workload count
    baseline list
    timing iterations
    index shape
    leaf policy
    max depth
    validation state
    weighted timing summary

The debug output should show:

    index validation
    sibling overlap metrics
    target retained leaf details
    target stage timing estimate
    target reconstruction timing estimate
    target retained execution phase estimate
    target retained allocation estimate
    target resultless timing estimate
    target count-only comparison

The count-only workload summary is generated from the workload CSV.

The count-only notes file is the human-readable count-only summary.

## Repeated low-gap trials

The review runner then generates repeated low-gap trials.

Repeated trial artifacts include:

    low-gap-trials-<label>/
    low-gap-trial-summary-<label>.csv
    low-gap-workload-trial-summary-<label>.csv
    low-gap-trial-details-<label>.csv
    low-gap-workload-trial-details-<label>.csv
    low-gap-trial-notes-<label>.txt

Repeated low-gap trials are the preferred owned-result evidence for low-selectivity movement.

Use these to review:

    aggregate low-selectivity timing movement
    candidate ratio movement
    avoided reconstruction movement
    weakest workload movement
    trial-to-trial stability

## Target workload review

Unless skipped, the review runner generates target workload artifacts.

Target workload artifacts include:

    target-workload-trial-details-<label>.csv
    target-workload-trial-summary-<label>.csv
    target-workload-trial-notes-<label>.txt

When previous target summaries are available, it also generates:

    target-workload-trial-comparison-<label>.csv

The target workload review is useful for focused boundary diagnostics.

Default target workloads:

    small:
      cluster_boundary_range

    large:
      large_cross_cluster_boundary

Target workload review should check:

    target workload name
    FSE average elapsed
    baseline average elapsed
    timing ratio
    visited nodes
    retained leaves
    reconstructed records
    matched records
    candidate ratio
    structural deltas versus previous run

## Count-only workload summary

The review runner generates count-only summary artifacts from the workload CSV.

Artifacts:

    count-only-workload-summary-<label>.csv
    count-only-workload-summary-<label>.txt

Use the TXT file for quick review.

Use the CSV file when machine-readable grouped metrics are needed.

The most important correctness field is:

    all_stats_match_owned

This must be true before count-only timing is used as evidence.

Count-only summary rows are grouped by:

    baseline
    selectivity bucket

The most useful count-only rows are usually low-selectivity rows.

Full-selectivity count-only rows can show very large speedups because root coverage can return cardinality without materializing all owned Vector results.

## Count-only comparison review

When a previous count-only workload summary is available, the review runner generates count-only comparison artifacts.

Artifacts:

    count-only-workload-summary-comparison-<label>.csv
    count-only-workload-summary-comparison-<label>.txt

The comparison notes should show:

    comparison rows
    missing previous rows
    missing current rows
    stats agreement
    low-selectivity speedup movement
    per-baseline low-selectivity movement

Correct comparison coverage:

    comparison rows: 9
    missing previous rows: 0
    missing current rows: 0

Expected stats agreement:

    all compared rows match owned-result structural stats in both summaries: true

Use count-only comparison artifacts only for count-only query mode movement.

Do not use count-only comparison artifacts to claim owned-result query speedups.

## Previous artifact resolution

When using:

    -PreviousLabel "<previous-label>"

the review runner resolves previous artifacts in this order:

    1. explicit previous artifact path, when provided
    2. flat OutputDir artifact path
    3. organized run folder at OutputDir/runs/<previous-label>/

Example flat path:

    benchmark_artifacts/count-only-workload-summary-previous-review.csv

Example organized path:

    benchmark_artifacts/runs/previous-review/count-only-workload-summary-previous-review.csv

If the flat artifact exists, it is used first.

If testing organized fallback specifically, remove or temporarily rename the flat previous artifact so the runner is forced to resolve from the organized run folder.

## Explicit previous artifact paths

The review runner supports explicit previous artifact inputs.

Use these when comparing across custom output directories or nonstandard artifact layouts.

Relevant previous artifact parameters include:

    -PreviousLowSelectivityGapCsv
    -PreviousTrialSummaryCsv
    -PreviousWorkloadSummaryCsv
    -PreviousCountOnlySummaryCsv
    -PreviousTargetSummaryCsv

Example:

    .\scripts\run-low-gap-review.ps1 `
      -Label "current-review" `
      -Dataset "large" `
      -PreviousCountOnlySummaryCsv "benchmark_artifacts\runs\previous-review\count-only-workload-summary-previous-review.csv" `
      -Trials 5 `
      -Iterations 10000

Explicit paths take precedence over previous-label lookup.

## Review notes

The review runner writes:

    low-gap-review-notes-<label>.txt

This is the main human-readable review artifact.

It should include:

    label
    previous label
    dataset
    max depth
    output directory
    organized run root
    previous organized run directory
    target workload
    copy-artifacts setting
    force-organized-artifacts setting
    previous input statuses
    benchmark status
    comparison statuses
    generated artifact paths
    decision guidance

Use review notes as the first file to inspect after a run.

## Review manifest

The review runner writes:

    low-gap-review-manifest-<label>.txt

The manifest records whether expected artifacts were found or skipped.

Manifest rows should distinguish:

    found
    skipped
    missing

Skipped artifacts should include a reason.

Examples:

    skipped | target workload comparison | previous target summary was not found
    skipped | count-only workload comparison CSV | previous count-only workload summary was not provided
    found | count-only workload summary | benchmark_artifacts\count-only-workload-summary-<label>.csv

Use the manifest to quickly verify artifact completeness.

## Comparison availability summary

The review notes include a comparison availability summary.

This section should explain whether each comparison completed or was skipped.

Expected comparison categories:

    single-run low-gap comparison
    aggregate repeated-trial comparison
    workload repeated-trial comparison
    count-only workload summary comparison
    target workload summary
    target workload comparison

Skipped comparisons are acceptable when the required previous artifacts do not exist.

Skipped comparisons are not acceptable when they were expected and the previous artifact path should have been available.

## Decision guidance

Review notes should end with decision guidance.

The guidance should remind the reviewer to use:

    aggregate comparison artifacts for broad low-gap timing movement
    workload comparison artifacts for weakest-workload movement
    target workload artifacts for target-specific movement
    count-only workload comparison artifacts for count-only query mode movement

Decision guidance should keep owned-result and count-only conclusions separate.

## Reporting-only commit review

Reporting-only commits should be judged by artifact correctness.

Keep a reporting-only commit when:

    validation passes
    expected artifacts are generated
    manifests are accurate
    CSVs are parseable
    notes are readable
    skipped comparisons have reasons
    existing workflows still work

Fix or revert a reporting-only commit when:

    required artifacts are missing
    CSVs are malformed
    review scripts fail
    manifests are misleading
    comparison artifacts silently skip required inputs
    previous-label resolution breaks

Timing movement alone is not a revert reason for a reporting-only commit.

## Performance commit review

Performance commits require repeated-trial evidence.

For owned-result performance changes, review:

    low-gap-trial-summary-<label>.csv
    low-gap-workload-trial-summary-<label>.csv
    target-workload-trial-summary-<label>.csv
    target-workload-trial-comparison-<label>.csv

For count-only performance changes, review:

    count-only-workload-summary-<label>.txt
    count-only-workload-summary-<label>.csv
    count-only-workload-summary-comparison-<label>.txt
    count-only-workload-summary-comparison-<label>.csv

A performance commit should explain:

    targeted benchmark mode
    dataset
    baseline
    selectivity bucket
    structural counter changes
    timing movement
    validation status
    acceptance or rejection reason

## Smoke-run interpretation

Smoke runs are useful for checking workflow correctness.

Typical smoke settings:

    Trials: 1
    Iterations: 1000

Smoke runs should be used to verify:

    scripts run
    validation passes
    artifacts are generated
    CSVs are parseable
    manifests are accurate

Smoke runs should not be used as final performance evidence.

Use repeated review settings for performance decisions:

    Trials: 5
    Iterations: 10000

or a larger review when needed.

## Standard checks

For docs-only or script-only commits:

    cargo test
    cargo check --release

For Rust code changes:

    cargo fmt
    cargo test
    cargo check --release

For benchmark-facing changes, also run at least one smoke review:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -Dataset "small" `
      -Trials 1 `
      -Iterations 1000

and, when relevant:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -Dataset "large" `
      -Trials 1 `
      -Iterations 1000

## Recommended full review flow

For a benchmark or query-performance commit:

    1. Run cargo fmt.
    2. Run cargo test.
    3. Run cargo check --release.
    4. Run the small dataset review if the small target or low-gap behavior may be affected.
    5. Run the large dataset review if larger-scale behavior may be affected.
    6. Inspect benchmark output for validation.
    7. Inspect review notes.
    8. Inspect review manifest.
    9. Inspect owned-result comparison artifacts.
    10. Inspect count-only comparison artifacts if count-only behavior is relevant.
    11. Decide keep, fix, revert, or gather more repeated-trial evidence.

## Current preferred review posture

Use small dataset review for:

    original boundary workload regression checks
    tiny-query behavior
    small fixed-cost behavior

Use large dataset review for:

    broader performance direction
    valid depth-16 target workload
    count-only versus owned-result output-contract behavior
    candidate-reduction evidence at larger scale

Use count-only artifacts for:

    exact-cardinality query mode
    result materialization overhead tracking
    output-contract comparisons

Use owned-result artifacts for:

    materialized row-return query performance
    baseline comparisons against flat scan, KD-tree, and R-tree
