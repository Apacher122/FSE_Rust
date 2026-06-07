# Benchmark Review Workflow

This file documents the benchmark review workflow for the FSE Rust implementation.

The review workflow is centered on:

    scripts/benchmark/review/run-low-gap-review.ps1

This script generates the normal benchmark artifact bundle, repeated low-gap trials, workload summaries, target workload summaries, count-only summaries, comparison artifacts, review notes, and a review manifest.

## Review purpose

The benchmark review workflow exists to keep performance decisions disciplined.

It should answer:

    did validation pass
    did owned-result FSE timing improve or regress
    did count-only timing improve or regress
    did existence timing or inspected-record behavior change
    did target workload behavior change
    did structural counters change
    did logical index footprint counters change
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

The review workflow currently distinguishes these output contracts:

    owned-result query execution:
      returns materialized Vec<Vector> results

    reference-result query execution:
      returns exact row references in a Vec<QueryResultReference>

    reference-visitor query execution:
      visits exact row references without constructing the reference Vec

    row-view query execution:
      visits borrowed exact row views without constructing owned Vector results

    count-only query execution:
      returns exact cardinality without owned Vector materialization

    existence query execution:
      returns whether the exact result set is non-empty

Owned-result, reference-result, reference-visitor, row-view, count-only, and existence conclusions are interpreted separately.

## Primary review command

New benchmark evidence should use run-number labels unless a legacy artifact name must be reproduced.

The review runner derives the label when `-RunNumber`, `-RunIndex`, and `-RunTopic` are provided without `-Label`:

    ```text
    run001-r01-reference-visitor-materialization-large
    ```

Label parts:

    ```text
    run001 = benchmark evidence iteration
    r01 = rerun index for the same evidence purpose
    reference-visitor-materialization = normalized topic
    large = dataset
    ```

Small dataset review:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -RunNumber 1 `
      -RunIndex 1 `
      -RunTopic "<topic>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "small" `
      -Trials 5 `
      -Iterations 10000

Large dataset review:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -RunNumber 1 `
      -RunIndex 1 `
      -RunTopic "<topic>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000

Large dataset review with organized artifact copy:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -RunNumber 1 `
      -RunIndex 1 `
      -RunTopic "<topic>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000 `
      -CopyArtifactsToRunFolder

Validated review with history update:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -RunNumber 1 `
      -RunIndex 1 `
      -RunTopic "<topic>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000 `
      -CopyArtifactsToRunFolder `
      -ValidateArtifacts `
      -UpdateHistory

If re-running the same label and intentionally replacing the organized copy:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -RunNumber 1 `
      -RunIndex 1 `
      -RunTopic "<topic>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000 `
      -CopyArtifactsToRunFolder `
      -ForceOrganizedArtifacts

`-Label` is still supported for legacy artifacts and explicit comparison work. New benchmark reviews should prefer the run-number parameters so the label is durable without chat context.

## Full comparison validation path

Use the full comparison validation path when the previous review run has every comparison input available.

A previous run should first be generated and copied to the organized run folder:

    ```powershell
    .\scripts\benchmark\review\run-low-gap-review.ps1 `
    -RunNumber 1 `
    -RunIndex 1 `
    -RunTopic "<previous-topic>" `
    -Dataset "small" `
    -Trials 5 `
    -Iterations 10000 `
    -CopyArtifactsToRunFolder `
    -ValidateArtifacts `
    -UpdateHistory
    ```

A current run can then validate the full previous/current comparison set by using `-PreviousLabel` and `-RequireValidatedComparisons`:

    ```powershell
    .\scripts\benchmark\review\run-low-gap-review.ps1 `
    -RunNumber 2 `
    -RunIndex 1 `
    -RunTopic "<current-topic>" `
    -PreviousLabel "<previous-label>" `
    -Dataset "small" `
    -Trials 5 `
    -Iterations 10000 `
    -CopyArtifactsToRunFolder `
    -ValidateArtifacts `
    -RequireValidatedComparisons `
    -UpdateHistory
    ```

`-RequireValidatedComparisons` is for the full comparison set:

    ```text
    single-run low-selectivity comparison
    aggregate trial comparison
    workload trial comparison
    count-only workload comparison
    materialization mode comparison
    target workload comparison
    ```

Do not use `-RequireValidatedComparisons` for a partial smoke unless all required previous artifacts are present.

If only target workload or materialization comparison is being smoke-tested, provide the specific previous summary path and omit `-RequireValidatedComparisons`. The validator still checks target/materialization equivalence when those comparison artifacts are present.

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

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -Label "<label>" `
      -Dataset "large" `
      -TargetWorkloadName "<target-workload-name>" `
      -Trials 5 `
      -Iterations 10000

Manual target summary command:

    .\scripts\benchmark\summarize\summarize-target-workload-trials.ps1 `
      -Label "<label>" `
      -InputLabel "<label>" `
      -TargetWorkloadName "<target-workload-name>" `
      -OutputDir "benchmark_artifacts"

## Manual max-depth override

Manual max-depth override is allowed only when intentionally testing build-depth behavior or comparing alternate valid build shapes.

Example:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
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
    index scalar footprint
    index footprint comparison against encoded coordinates
    leaf policy
    max depth
    validation state
    weighted timing summary

The debug output should show:

    index validation
    index scalar footprint
    index footprint comparison against encoded coordinates
    sibling overlap metrics
    target retained leaf details
    target stage timing estimate
    target reconstruction timing estimate
    target retained execution phase estimate
    target retained allocation estimate
    target resultless timing estimate
    target materialization mode comparison
    workload materialization mode summary

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

Unless skipped, the review runner generates target workload artifacts from the repeated trial workload CSVs.

Target workload artifacts include:

    target-workload-trial-details-<label>.csv
    target-workload-trial-summary-<label>.csv
    target-workload-trial-notes-<label>.txt

When previous target summaries are available, it also generates:

    target-workload-trial-comparison-<label>.csv
    target-workload-trial-comparison-<label>.txt

The target workload review is useful for focused boundary diagnostics.

Default target workloads:

    small:
      cluster_boundary_range

    large:
      large_cross_cluster_boundary

The target workload summary is grouped by tree baseline:

    kd_tree
    r_tree

Target workload review should check:

    target workload name
    trial count
    FSE average elapsed
    baseline average elapsed
    timing ratio
    count-only average elapsed
    reference-result average elapsed
    reusable owned average elapsed
    visited nodes
    retained leaves
    reconstructed records
    matched records
    candidate ratio
    structural deltas versus previous run
    stats-equivalence fields

Required stats-equivalence fields:

    all_count_only_stats_match_owned
    all_reference_stats_match_count_only
    all_reusable_owned_stats_match_owned

These must be true before target workload timing movement is used as evidence.

The review validator enforces these fields when target workload review artifacts are present. If any field is false, the review artifact validation must fail even if all files exist.

If `-SkipTargetWorkloadReview` is used, target workload artifacts should be marked skipped in the manifest rather than missing.

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

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
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
    run id
    attempt id
    run topic
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
    history update status when requested
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

## History ledgers

The review runner can append validated review results into long-lived history CSVs.

History updates are optional and use:

    ```powershell
    -CopyArtifactsToRunFolder `
    -ValidateArtifacts `
    -UpdateHistory
    ```

`-UpdateHistory` requires both organized raw artifacts and validation. This prevents failed or temporary smoke runs from entering the history ledgers.

The default history directory is:

    ```text
    benchmark_artifacts/history/
    ```

History CSVs:

    ```text
    benchmark-run-history.csv
    low-selectivity-performance-history.csv
    weakest-workload-history.csv
    count-only-workload-history.csv
    materialization-mode-history.csv
    target-workload-history.csv
    index-footprint-history.csv
    baseline-footprint-history.csv
    ```

The per-run artifacts remain the raw evidence. History CSVs are append-only ledgers for inspection across runs. A history row is not a replacement for the raw artifact named in its `source_csv` column.

`index-footprint-history.csv` is generated from the normal summary CSV. It stores one deduplicated index-footprint row per validated review run because the index footprint is the same across baseline rows for a given run.

`baseline-footprint-history.csv` is generated from the normal summary CSV. It stores one row per baseline per validated review run because each baseline has its own logical scalar footprint.

Every summary history row includes:

    ```text
    run_id
    attempt_id
    run_label
    run_topic
    run_dataset
    run_max_depth
    run_trials
    run_iterations
    run_target_workload
    source_csv
    ```

Then it preserves the source artifact columns from the corresponding summary CSV.

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
    existence timing summaries and debug output for exact non-empty-result behavior

Decision guidance should separate owned-result, reference-result, reference-visitor, row-view, count-only, and existence conclusions.

## Reporting-only commit review

Reporting-only commits should be judged by artifact correctness.

Keep a reporting-only commit when:

    validation passes
    expected artifacts are generated
    new footprint fields are populated when the commit changes footprint reporting
    manifests are accurate
    CSVs are parseable
    notes are readable
    skipped comparisons have reasons
    existing workflows still work

Fix or revert a reporting-only commit when:

    required artifacts are missing
    CSVs are malformed
    footprint fields are missing after a footprint-reporting change
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

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
      -Label "<label>" `
      -Dataset "small" `
      -Trials 1 `
      -Iterations 1000

and, when relevant:

    .\scripts\benchmark\review\run-low-gap-review.ps1 `
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

## Materialization mode review artifact

The review workflow now preserves the debug report's workload materialization mode summary as a CSV artifact.

This artifact is separate from the count-only workload summary. Count-only summaries answer whether exact cardinality avoids owned-result materialization. Materialization mode summaries compare six output contracts for every workload:

    ```text
    fresh owned result
    reusable owned result
    reference-result
    reference-visitor
    row-view
    count-only
    ```

The review rule is:

    ```text
    Every materialization mode summary row must have agreement = pass.
    ```

Use this artifact to identify whether owned result materialization, reusable output buffers, reference-result construction, reference-visitor dispatch, row-view delivery, or count-only execution is the current output-contract cost driver. Do not use it as a standalone performance claim without repeated-trial evidence.

## Repeated materialization mode trials

Use repeated materialization mode trials when a single materialization summary is too noisy to decide whether an output contract deserves more work.

Command:

    ```powershell
    .\scripts\benchmark\run\run-materialization-mode-trials.ps1 `
    -RunNumber 3 `
    -RunIndex 1 `
    -RunTopic "row view materialization trials" `
    -Dataset "small" `
    -Trials 5 `
    -Iterations 1000
    ```

Generated artifacts:

    ```text
    materialization-mode-trials-<label>/
    materialization-mode-trial-details-<label>.csv
    materialization-mode-trial-summary-<label>.csv
    materialization-mode-trial-notes-<label>.txt
    ```

The trial directory stores each trial debug output and each per-trial materialization summary extracted from that debug output. The detail CSV is the row-level repeated evidence. The summary CSV groups by workload and reports mean elapsed times, row-view stability fields, row-view faster-than counts, and agreement counts.

Repeated materialization summaries are output-contract evidence. They do not replace the normal benchmark review workflow, low-gap repeated trials, target workload review, count-only review, or existence timing review.

Required interpretation rule:

    ```text
    all_agreement_pass must be true before timing movement is used for a workload.
    ```

Use repeated materialization trials to decide whether row-view, reference-visitor, reference-result, reusable-owned, owned-result, or count-only delivery looks worth deeper design work. Do not use them to claim geometric pruning improved.

## Current row-view materialization decision

The first small and large repeated row-view materialization trials used:

    ```text
    run003-r01-row-view-materialization-trials-small
    run003-r01-row-view-materialization-trials-large
    ```

Both runs used 5 trials and 1000 iterations. The small summary loaded 30 detail rows across 6 workloads. The large summary loaded 65 detail rows across 13 workloads. Both summaries reported zero agreement failures.

Small result:

    ```text
    row-view beat reference-result on the selective cluster workloads
    row-view did not beat reference-visitor on any workload
    full_dataset_range favored reference-visitor and count-only over row-view
    ```

Large result:

    ```text
    row-view did not beat reference-visitor on any matched workload
    large_cross_cluster_boundary: 532.2ns row-view, 381.0ns reference-visitor
    large_full_dataset_range: 18.6414us row-view, 3.9052us reference-visitor
    ```

Decision:

    ```text
    keep row-view as an exact borrowed output contract
    do not expand row-view for performance work based on current evidence
    prefer reference-visitor for streaming exact match delivery
    pause batch row-view delivery and reference-list row-view iteration unless a caller requirement changes the priority
    ```

## Reference visitor materialization result

The first repeated small-dataset materialization review for the reference visitor used:

    ```text
    label: run001-r01-reference-visitor-materialization-small
    dataset: small
    trials: 5
    iterations: 1000
    ```

The materialization summary reported `agreement = pass` for every workload.

The reference visitor was faster than the reference-result path on 5 of 6 workloads. The only slower workload was `empty_far_range`, where the visitor was 1ns slower and should be treated as noise-equivalent.

Target workload result for `cluster_boundary_range`:

    ```text
    fresh owned average elapsed: 243ns
    reusable owned average elapsed: 134ns
    reference-result average elapsed: 146ns
    reference-visitor average elapsed: 105ns
    count-only average elapsed: 104ns
    ```

The paired large-dataset materialization review used:

    ```text
    label: run001-r01-reference-visitor-materialization-large
    dataset: large
    trials: 5
    iterations: 1000
    ```

Large materialization result:

    ```text
    visitor faster than reference-result: 11 of 13 workloads
    visitor equal to reference-result: 1 of 13 workloads
    visitor slower than reference-result: 1 of 13 workloads
    large_full_dataset_range reference-result: 11.656us
    large_full_dataset_range reference-visitor: 5.625us
    large_full_dataset_range visitor advantage: 6.031us
    ```

Review interpretation:

    ```text
    reference visitor reduces reference-result output overhead on the current small and large materialization summaries
    reference visitor is not a replacement for count-only resultless execution
    this is output-contract evidence, not geometric pruning evidence
    large_full_dataset_range is the strongest current visitor result because it avoids collecting 10000 references into a Vec
    ```

## Materialization summary comparison

Materialization summary comparison is a single-run artifact review helper. It compares the extracted materialization-mode summary CSV from one run against another run.

Command:

    ```powershell
    .\scripts\benchmark\compare\compare-materialization-mode-summary.ps1 `
    -Label "<label>" `
    -PreviousMaterializationSummaryCsv "benchmark_artifacts\materialization-mode-summary-<previous>.csv" `
    -CurrentMaterializationSummaryCsv "benchmark_artifacts\materialization-mode-summary-<current>.csv" `
    -OutputDir "benchmark_artifacts"
    ```

The comparison output is:

    ```text
    materialization-mode-summary-comparison-<label>.csv
    materialization-mode-summary-comparison-<label>.txt
    ```

Review rules:

    ```text
    agreement_status must remain pass
    matched_records_delta should be 0
    elapsed classifications are review signals only
    speedup classifications are review signals only
    ```

Do not treat a single materialization comparison as a performance proof. Use it to identify which output contract moved, then confirm with repeated benchmark evidence when a performance claim matters.

## Target comparison validation scope

Target workload summary equivalence is validated whenever target summary artifacts are found.

Target workload comparison equivalence is validated whenever target comparison artifacts are found.

The `-RequireValidatedComparisons` flag still applies to the broader comparison set, including aggregate trial, workload trial, and count-only workload comparisons. Do not use that flag for a target-only comparison smoke unless the full previous comparison input set is also provided.

## Materialization agreement validation

Materialization review artifacts are part of the benchmark evidence contract.

The review validator requires:

    ```text
    materialization-mode-summary-<label>.csv
    agreement = pass

    materialization-mode-summary-comparison-<label>.csv
    agreement_status = pass
    ```

The comparison artifact is validated whenever it is present. It does not require the full comparison set unless `-RequireValidatedComparisons` is explicitly used.

A timing improvement is not review-grade evidence if owned, reusable-owned, reference-result, reference-visitor, row-view, and count-only output contracts disagree.

## Validation coverage reference

Use `validation-coverage.md` as the source of truth for review validator scope.

It defines:

    ```text
    required artifacts
    required CSV columns
    index-validity gates
    count-only equivalence gates
    materialization agreement gates
    target workload equivalence gates
    optional comparison behavior
    ```

The review workflow should not treat timing regression as a validation failure by itself. Timing movement is review evidence. Validation failures are reserved for missing artifacts, malformed artifact contracts, invalid index state, or semantic/equivalence drift.
