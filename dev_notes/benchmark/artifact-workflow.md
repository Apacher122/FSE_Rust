# Benchmark Artifact Workflow

This file documents how benchmark artifacts are organized, copied, resolved, and cleaned up.

The benchmark review workflow can generate many files per run. Artifact organization is a storage-management concern only. It does not change benchmark execution, timing, validation, query behavior, or comparison semantics.

## Artifact layout

The flat artifact root is:

```text
benchmark_artifacts/
```

The organized run layout is:

```text
benchmark_artifacts/
  history/
    benchmark-run-history.csv
    low-selectivity-performance-history.csv
    weakest-workload-history.csv
    count-only-workload-history.csv
    materialization-mode-history.csv
    typed-indexed-comparison-history.csv
    typed-archive-load-history.csv
    typed-archive-append-rebuild-history.csv
    typed-archive-append-delta-query-history.csv
    typed-archive-compaction-history.csv
    typed-archive-maintenance-history.csv
    typed-archive-append-delta-maintenance-history.csv
    target-workload-history.csv
    index-footprint-history.csv
    baseline-footprint-history.csv
  runs/
    <label>/
      benchmark-output-<label>.txt
      debug-output-<label>.txt
      summary-<label>.csv
      workloads-<label>.csv
      count-only-workload-summary-<label>.csv
      count-only-workload-summary-<label>.txt
      materialization-mode-summary-<label>.csv
      materialization-mode-summary-<label>.txt
      materialization-mode-trial-details-<label>.csv
      materialization-mode-trial-summary-<label>.csv
      materialization-mode-trial-notes-<label>.txt
      materialization-mode-trials-<label>/
      existence-timing-summary-<label>.csv
      existence-timing-summary-<label>.txt
      typed-indexed-comparison-summary-<label>.csv
      typed-indexed-comparison-summary-<label>.txt
      typed-archive-load-summary-<label>.csv
      typed-archive-load-summary-<label>.txt
      typed-archive-append-rebuild-summary-<label>.csv
      typed-archive-append-rebuild-summary-<label>.txt
      typed-archive-append-delta-query-summary-<label>.csv
      typed-archive-append-delta-query-summary-<label>.txt
      typed-archive-compaction-summary-<label>.csv
      typed-archive-compaction-summary-<label>.txt
      typed-archive-maintenance-summary-<label>.csv
      typed-archive-maintenance-summary-<label>.txt
      typed-archive-append-delta-maintenance-summary-<label>.csv
      typed-archive-append-delta-maintenance-summary-<label>.txt
      typed-query-index-archive-<label>.fse
      low-gap-review-notes-<label>.txt
      low-gap-review-manifest-<label>.txt
      low-gap-trials-<label>/
```

The flat root is useful for active comparison workflows.

The organized run folder is useful for long-term inspection and cleanup.

The history folder is useful for cross-run inspection. It is not the raw evidence source.

## Run-number label convention

New benchmark labels should use run-number nomenclature:

```text
run001-r01-reference-visitor-materialization-large
```

The parts are:

```text
run001: benchmark evidence iteration
r01: rerun index for the same evidence purpose
reference-visitor-materialization: topic
large: dataset
```

Generate this label through the review runner:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -RunNumber 1 `
  -RunIndex 1 `
  -RunTopic "reference visitor materialization" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 1000 `
  -CopyArtifactsToRunFolder `
  -ValidateArtifacts `
  -UpdateHistory
```

`-Label` remains supported for legacy artifacts and explicit reproduction work. New evidence runs should prefer `-RunNumber`, `-RunIndex`, and `-RunTopic`.

## Benchmark artifact organization workflow

Benchmark reviews can produce many artifacts per run. The flat `benchmark_artifacts` directory is useful for active comparison workflows, but it becomes difficult to inspect once many labels have accumulated.

The artifact organization helper is:

```text
scripts/benchmark/artifacts/organize-benchmark-artifacts.ps1
```

It groups recognized benchmark artifacts by label into:

```text
benchmark_artifacts/runs/<label>/
```

Example organized layout:

```text
benchmark_artifacts/
  runs/
    count-only-summary-notes-large-smoke/
      benchmark-output-count-only-summary-notes-large-smoke.txt
      debug-output-count-only-summary-notes-large-smoke.txt
      workloads-count-only-summary-notes-large-smoke.csv
      count-only-workload-summary-count-only-summary-notes-large-smoke.csv
      count-only-workload-summary-count-only-summary-notes-large-smoke.txt
      low-gap-review-notes-count-only-summary-notes-large-smoke.txt
      low-gap-review-manifest-count-only-summary-notes-large-smoke.txt
```

The safest first use is dry-run mode:

```powershell
.\scripts\benchmark\artifacts\organize-benchmark-artifacts.ps1 -DryRun
```

The recommended normal use is copy mode:

```powershell
.\scripts\benchmark\artifacts\organize-benchmark-artifacts.ps1 -Copy
```

Copy mode creates organized folders while preserving the flat artifact root. This is the safest mode when a label is still being actively inspected or used as a comparison baseline.

Use move mode only when old artifacts no longer need to remain in the flat root:

```powershell
.\scripts\benchmark\artifacts\organize-benchmark-artifacts.ps1
```

Organize a single label with:

```powershell
.\scripts\benchmark\artifacts\organize-benchmark-artifacts.ps1 `
  -Label "<label>" `
  -Copy
```

Use `-Force` only when intentionally overwriting already-organized artifacts:

```powershell
.\scripts\benchmark\artifacts\organize-benchmark-artifacts.ps1 `
  -Copy `
  -Force
```

The organization script recognizes normal benchmark artifacts, low-gap artifacts, target workload artifacts, count-only artifacts, materialization-mode artifacts, repeated materialization trial artifacts, existence timing artifacts, typed query timing artifacts, review manifests, review notes, and repeated-trial folders.

Unrecognized files are left in place. This prevents accidental movement of manually-created notes, temporary files, or unrelated data.

## Organized previous-label resolution

The review runner can resolve previous-label artifacts from both the flat artifact directory and organized run folders.

When a review command uses:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "current-review" `
  -PreviousLabel "previous-review" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000
```

the previous artifact lookup order is:

```text
1. explicit previous artifact path, when provided
2. flat OutputDir artifact path
3. organized run folder at OutputDir/runs/<previous-label>/
```

Example flat artifact path:

```text
benchmark_artifacts/count-only-workload-summary-previous-review.csv
```

Example organized artifact path:

```text
benchmark_artifacts/runs/previous-review/count-only-workload-summary-previous-review.csv
```

The review runner checks the flat artifact path first. If the flat artifact exists, it uses that path even if an organized copy also exists.

This means a smoke run with both flat and organized copies proves that flat previous-label resolution still works, but it does not prove organized fallback by itself.

To prove organized fallback, the flat previous artifacts must be absent or the review must use an `OutputDir` where only the organized run folder exists.

Safe fallback test pattern:

```text
1. Copy the previous label into benchmark_artifacts/runs/<previous-label>/.
2. Move or rename the flat previous artifacts temporarily.
3. Run the review with -PreviousLabel "<previous-label>".
4. Confirm the review notes show resolved paths under benchmark_artifacts/runs/<previous-label>/.
5. Restore the flat artifacts if needed.
```

Example organization command:

```powershell
.\scripts\benchmark\artifacts\organize-benchmark-artifacts.ps1 `
  -Label "previous-review" `
  -Copy `
  -Force
```

Example fallback review command:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "organized-fallback-smoke" `
  -PreviousLabel "previous-review" `
  -Dataset "large" `
  -Trials 1 `
  -Iterations 1000
```

The review notes and manifest include:

```text
Organized run root: benchmark_artifacts\runs
Previous organized run directory: benchmark_artifacts\runs\<previous-label>
```

When fallback succeeds, the previous input resolved paths should point into:

```text
benchmark_artifacts\runs\<previous-label>\
```

Do not assume organized fallback was tested if the resolved previous paths still point to the flat artifact root.

Recommended usage:

```text
use flat artifacts for active baselines
use organized copies for easier inspection
use organized fallback for archived baselines
use explicit previous artifact paths when comparing across custom OutputDir layouts
```

This keeps old benchmark labels usable after artifact cleanup while preserving the simple flat-folder workflow for active development.

## Optional organized artifact copy during review

The review runner can optionally copy the current review artifacts into an organized run folder after the review completes.

The default behavior remains unchanged:

```text
benchmark artifacts are written to the flat OutputDir
```

The optional organized copy behavior also writes a copy to:

```text
OutputDir/runs/<label>/
```

Use this when a run should remain easy to inspect by label while preserving the flat artifact layout for active comparisons.

Default review command:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000
```

Organized-copy review command:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000 `
  -CopyArtifactsToRunFolder
```

If re-running the same label and intentionally overwriting the organized copy, use:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000 `
  -CopyArtifactsToRunFolder `
  -ForceOrganizedArtifacts
```

The organized copy is useful because it creates a per-run folder like:

```text
benchmark_artifacts/
  runs/
    <label>/
      benchmark-output-<label>.txt
      debug-output-<label>.txt
      summary-<label>.csv
      workloads-<label>.csv
      count-only-workload-summary-<label>.csv
      count-only-workload-summary-<label>.txt
      materialization-mode-summary-<label>.csv
      materialization-mode-summary-<label>.txt
      materialization-mode-trial-details-<label>.csv
      materialization-mode-trial-summary-<label>.csv
      materialization-mode-trial-notes-<label>.txt
      materialization-mode-trials-<label>/
      existence-timing-summary-<label>.csv
      existence-timing-summary-<label>.txt
      typed-indexed-comparison-summary-<label>.csv
      typed-indexed-comparison-summary-<label>.txt
      typed-archive-load-summary-<label>.csv
      typed-archive-load-summary-<label>.txt
      typed-archive-append-rebuild-summary-<label>.csv
      typed-archive-append-rebuild-summary-<label>.txt
      typed-archive-append-delta-query-summary-<label>.csv
      typed-archive-append-delta-query-summary-<label>.txt
      typed-archive-compaction-summary-<label>.csv
      typed-archive-compaction-summary-<label>.txt
      typed-archive-maintenance-summary-<label>.csv
      typed-archive-maintenance-summary-<label>.txt
      typed-archive-append-delta-maintenance-summary-<label>.csv
      typed-archive-append-delta-maintenance-summary-<label>.txt
      typed-query-index-archive-<label>.fse
      low-gap-review-notes-<label>.txt
      low-gap-review-manifest-<label>.txt
      low-gap-trials-<label>/
```

The review notes record whether organized copying was requested:

```text
Copy artifacts to run folder: True
Force organized artifacts: False
```

When organized copying is enabled, the review notes also include:

```text
Organized artifact copy
-----------------------
status: running
destination: benchmark_artifacts\runs\<label>
force: False
status: completed
```

The organized copy should include the final completed review notes. If checking this manually, confirm that both files contain `status: completed`:

```powershell
Select-String `
  -Path .\benchmark_artifacts\low-gap-review-notes-<label>.txt `
  -Pattern "status: completed"

Select-String `
  -Path .\benchmark_artifacts\runs\<label>\low-gap-review-notes-<label>.txt `
  -Pattern "status: completed"
```

Recommended usage:

```text
use flat artifacts for active previous-label comparisons
use -CopyArtifactsToRunFolder for every review that should be easy to inspect later
use -ForceOrganizedArtifacts only when intentionally replacing an organized copy
use the standalone organize script for older artifacts that were generated before organized copy support
```

The organized-copy option is a convenience feature. It does not change benchmark execution, timing, validation, or comparison behavior.

## History CSV workflow

Raw artifacts remain split by run and label. History CSVs are append-only ledgers that make repeated evidence easier to inspect without opening every run folder.

History updates are generated by the review runner when all of these are provided:

```powershell
-CopyArtifactsToRunFolder `
-ValidateArtifacts `
-UpdateHistory
```

The history update writes to:

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
typed-indexed-comparison-history.csv
typed-archive-load-history.csv
typed-archive-append-rebuild-history.csv
typed-archive-append-delta-query-history.csv
typed-archive-compaction-history.csv
typed-archive-maintenance-history.csv
typed-archive-append-delta-maintenance-history.csv
target-workload-history.csv
index-footprint-history.csv
baseline-footprint-history.csv
```

The history files are intentionally named for the question they answer:

```text
benchmark-run-history.csv:
  which validated review runs exist

low-selectivity-performance-history.csv:
  how aggregate low-selectivity timing changed across runs

weakest-workload-history.csv:
  which workload was weakest in repeated trials

count-only-workload-history.csv:
  how exact cardinality execution changed across buckets

materialization-mode-history.csv:
  how output-contract materialization costs changed by workload

typed-indexed-comparison-history.csv:
  how typed planner strategy, selectivity bucket, planner risk, planner-vs-flat-scan cost deltas, candidate ratio, and indexed timing changed by workload

typed-archive-load-history.csv:
  how loaded `.fse` query execution changed by workload

typed-archive-append-rebuild-history.csv:
  how append/rebuild archive timing and archive growth changed by workload

typed-archive-append-delta-query-history.csv:
  how base-plus-append-delta query execution compared with rebuilt query execution by workload

typed-archive-compaction-history.csv:
  how tombstone compaction changed logical archive bytes and query result counts by workload

typed-archive-maintenance-history.csv:
  how policy-selected archive maintenance changed logical archive bytes and query result counts by workload

typed-archive-append-delta-maintenance-history.csv:
  how persisted append-delta maintenance changed logical archive bytes and query result counts by workload

target-workload-history.csv:
  how the selected boundary workload changed across baselines

index-footprint-history.csv:
  how logical index scalar footprint and scalar payload byte estimates changed across validated runs

baseline-footprint-history.csv:
  how logical baseline scalar footprint and shared index byte estimates changed across validated runs
```

Each summary history row starts with run metadata:

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

The remaining columns are copied from the source summary CSV.

Do not edit history rows manually to make a benchmark look cleaner. If a run should not count, leave the raw artifact alone and document the decision in benchmark notes.

## Index footprint history

The review runner writes one deduplicated footprint row per validated review run to:

```text
benchmark_artifacts/history/index-footprint-history.csv
```

The source is the normal summary CSV for the run. Because the summary CSV has one row per baseline, the history writer keeps the unique index-footprint row instead of repeating identical footprint values for every baseline.

The footprint history keeps these groups of fields:

```text
run metadata
dataset and benchmark configuration
index shape
leaf cardinality
index density and volume
encoded coordinate scalar count
residual scalar count
centroid scalar count
bounds scalar count
structural metadata scalar count
total counted index scalar count
scalar payload byte estimate fields
encoded-baseline comparison ratios
index validation fields
```

Footprint history values include logical scalar counts and scalar payload byte estimates. Byte estimate fields multiply scalar counts by scalar width. They do not measure allocator overhead, physical layout overhead, compression ratio, cache behavior, file size, or memory-mapped persistence size.

## Baseline footprint history

The review runner writes one footprint row per baseline per validated review run to:

```text
benchmark_artifacts/history/baseline-footprint-history.csv
```

The source is the normal summary CSV for the run. Because the summary CSV has one row per baseline, the history writer preserves baseline identity fields and baseline footprint fields instead of collapsing the rows into one index-level record.

The baseline footprint history keeps these groups of fields:

```text
run metadata
dataset and benchmark configuration
index scalar comparison fields
index scalar payload byte estimate fields
index validation fields
baseline identity
baseline node counts
baseline point-coordinate scalar count
baseline routing metadata scalar count
baseline bounds metadata scalar count
baseline structural metadata scalar count
baseline total counted scalar count
baseline point and structural ratios
```

Baseline footprint history values include baseline logical scalar counts and the shared FSE index scalar payload byte estimates for the run. Byte estimate fields multiply scalar counts by scalar width. They do not measure allocator overhead, physical layout overhead, compression ratio, cache behavior, file size, or memory-mapped persistence size.

## Flat benchmark artifact cleanup workflow

The flat `benchmark_artifacts` directory is useful for active development, but it can become crowded after repeated review runs.

The cleanup helper is:

```text
scripts/benchmark/artifacts/cleanup-flat-benchmark-artifacts.ps1
```

This script removes flat artifacts only when the same artifact already exists in the organized run folder.

Example flat artifact:

```text
benchmark_artifacts/debug-output-my-review.txt
```

Required organized copy:

```text
benchmark_artifacts/runs/my-review/debug-output-my-review.txt
```

The cleanup script is safe by default. Without `-Delete`, it only performs a dry run:

```powershell
.\scripts\benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1
```

Preview cleanup for one label:

```powershell
.\scripts\benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1 `
  -Label "my-review"
```

Delete eligible flat artifacts for one label:

```powershell
.\scripts\benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1 `
  -Label "my-review" `
  -Delete `
  -Force
```

Delete all eligible flat artifacts that already have organized copies:

```powershell
.\scripts\benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1 `
  -Delete `
  -Force
```

Recommended safe cleanup flow:

```text
1. Run the benchmark review normally.
2. Copy artifacts into organized run folders.
3. Dry-run flat cleanup.
4. Delete only after confirming the dry-run output.
```

Commands:

```powershell
.\scripts\benchmark\artifacts\organize-benchmark-artifacts.ps1 -Copy

.\scripts\benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1

.\scripts\benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1 `
  -Delete `
  -Force
```

The cleanup script recognizes the same normal benchmark artifact families as the organization script, including:

```text
benchmark output
debug output
summary CSV
workloads CSV
low-selectivity gap CSV
low-gap review notes
low-gap review manifest
low-gap trial artifacts
target workload artifacts
count-only workload summaries
count-only comparison artifacts
typed indexed comparison summaries
typed archive load summaries
typed archive append rebuild summaries
typed archive append-delta query summaries
typed archive compaction summaries
typed archive maintenance summaries
typed archive append-delta maintenance summaries
typed query index archives
repeated-trial folders
repeated materialization trial folders
```

Artifacts are skipped when no organized copy exists.

Unrecognized top-level files are also skipped. This prevents accidental deletion of manually-created notes, temporary files, or unrelated data.

Important rule:

```text
cleanup only removes flat artifacts that already exist under benchmark_artifacts/runs/<label>/
```

Flat artifact cleanup is a storage-management step only. It does not change benchmark evidence, benchmark validation, query behavior, or comparison semantics.

Use cleanup when:

```text
the flat benchmark_artifacts folder is too crowded
the label already has an organized copy
the label is not needed in the flat root for immediate manual inspection
```

Avoid cleanup when:

```text
the artifacts have not been copied into benchmark_artifacts/runs/<label>/
the label is still being actively inspected in the flat root
you are not sure which artifacts are active comparison baselines
```

Previous-label comparison compatibility:

```text
the review runner can resolve previous artifacts from the flat root or organized run folders
```

So cleanup is safe for archived labels once organized copies exist. For active labels, keeping both flat and organized copies may still be more convenient.

## Recommended artifact workflow

For active benchmark development:

```text
1. Run the review normally.
2. Inspect benchmark output, debug output, review notes, and manifest.
3. Use the flat artifact root for immediate comparisons.
4. Use -CopyArtifactsToRunFolder when a run should be easy to inspect later.
```

For older runs:

```text
1. Copy artifacts into organized folders.
2. Confirm organized copies exist.
3. Dry-run cleanup.
4. Delete flat copies only after confirming the dry run.
```

For archived baselines:

```text
1. Keep organized run folders.
2. Use previous-label fallback or explicit previous artifact paths.
3. Do not rely on the flat artifact root unless the files still exist there.
```

## Artifact workflow rules

Do not use artifact organization as benchmark evidence.

Do not delete flat artifacts unless organized copies exist.

Do not assume organized fallback was tested if flat previous artifacts still exist.

Use `-Copy` before cleanup when preserving safety matters.

Use `-Force` only when intentionally overwriting existing organized copies or deleting eligible flat artifacts.

Keep artifact-management changes separate from query-performance changes.

## Materialization mode summary artifact

The debug report includes a workload materialization mode summary. The benchmark artifact workflow should keep a machine-readable copy of that section so owned-result, reusable-owned, reference-result, reference-visitor, row-view, and count-only timings can be reviewed without manually scraping terminal output.

The summary artifact is generated from the debug report:

```powershell
.\scripts\benchmark\summarize\summarize-materialization-mode-summary.ps1 `
  -DebugOutputPath "benchmark_artifacts\debug-output-<label>.txt" `
  -OutputCsv "benchmark_artifacts\materialization-mode-summary-<label>.csv" `
  -OutputNotesPath "benchmark_artifacts\materialization-mode-summary-<label>.txt"
```

The generated CSV uses these columns:

```text
dataset
max_depth
workload_name
matched_records
fresh_owned_elapsed
reusable_owned_elapsed
reference_elapsed
visitor_elapsed
row_view_elapsed
count_only_elapsed
count_speedup
reference_speedup
visitor_speedup
row_view_speedup
reusable_speedup
agreement
```

The agreement column must remain `pass` for every workload before using materialization timing as evidence.

## Materialization summary artifacts

Materialization-mode summaries are extracted from benchmark debug output by:

```text
scripts/benchmark/summarize/summarize-materialization-mode-summary.ps1
```

The normal benchmark script writes:

```text
materialization-mode-summary-<label>.csv
materialization-mode-summary-<label>.txt
```

The CSV stores per-workload timing for:

```text
fresh owned
reusable owned
reference-result
reference-visitor
row-view
count-only
```

It also stores the dataset, effective max depth, matched record count, speedup ratios, and an agreement column.

Use the comparison helper when comparing two materialization summary CSVs:

```powershell
.\scripts\benchmark\compare\compare-materialization-mode-summary.ps1 `
  -Label "<label>" `
  -PreviousMaterializationSummaryCsv "benchmark_artifacts\materialization-mode-summary-<previous>.csv" `
  -CurrentMaterializationSummaryCsv "benchmark_artifacts\materialization-mode-summary-<current>.csv" `
  -OutputDir "benchmark_artifacts"
```

The comparison helper writes:

```text
materialization-mode-summary-comparison-<label>.csv
materialization-mode-summary-comparison-<label>.txt
```

Use these artifacts to review materialization movement separately from exactness agreement. A row can have noisy timing movement while still preserving exact result semantics.

## Materialization mode artifacts

Normal benchmark runs now produce materialization-mode evidence alongside the existing count-only workload summary.

Generated files:

```text
materialization-mode-summary-<label>.csv
materialization-mode-summary-<label>.txt
materialization-mode-summary-comparison-<label>.csv
materialization-mode-summary-comparison-<label>.txt
```

The summary CSV records workload-level timing for fresh owned, reusable owned, reference-result, reference-visitor, row-view, and count-only query output modes. The comparison CSV compares those output-contract costs across review runs.

Every materialization summary row should report `agreement = pass`; otherwise the output modes are not preserving the same exact query semantics.

## Repeated materialization trial artifacts

Repeated materialization trial artifacts are generated by:

```text
scripts/benchmark/run/run-materialization-mode-trials.ps1
```

The runner executes repeated debug reports and extracts a materialization summary for each trial.

New evidence runs should use run-number label parameters:

```powershell
.\scripts\benchmark\run\run-materialization-mode-trials.ps1 `
  -RunNumber 3 `
  -RunIndex 1 `
  -RunTopic "row view materialization trials" `
  -Dataset "small" `
  -Trials 5 `
  -Iterations 1000
```

Generated files:

```text
materialization-mode-trials-<label>/
materialization-mode-trial-details-<label>.csv
materialization-mode-trial-summary-<label>.csv
materialization-mode-trial-notes-<label>.txt
```

The trial directory contains per-trial debug output and per-trial materialization summary CSV/TXT files. The detail CSV preserves every trial/workload row in nanoseconds. The summary CSV groups by workload and records:

```text
trial_count
matched_records
agreement_pass_count
all_agreement_pass
mean_fresh_owned_elapsed_ns
mean_reusable_owned_elapsed_ns
mean_reference_elapsed_ns
mean_visitor_elapsed_ns
mean_row_view_elapsed_ns
mean_count_only_elapsed_ns
row-view faster-than counts
```

The required correctness gate is:

```text
all_agreement_pass = true
```

Repeated materialization trial artifacts are useful when deciding whether an output-contract direction is worth expanding. They are not raw FSE pruning evidence and should not be mixed with low-gap owned-result evidence.

## Existence timing artifacts

Existence timing artifacts are extracted from the debug report workload summary.

Artifacts:

```text
existence-timing-summary-<label>.csv
existence-timing-summary-<label>.txt
```

The CSV preserves the workload-level relationship between owned-result presence, count-only presence, and exact existence. It also records count-only candidate records, existence inspected records, skipped candidate records, and the existence inspection ratio.

Required CSV fields include:

```text
dataset
max_depth
workload_name
owned_has_match
count_only_has_match
existence_has_match
count_only_candidate_records
existence_inspected_records
existence_skipped_records
existence_inspection_ratio
fresh_owned_elapsed
count_only_elapsed
existence_elapsed
existence_speedup_vs_owned
existence_speedup_vs_count_only
agreement
```

Every row should have:

```text
agreement = pass
```

The notes file records row count, agreement failures, aggregate candidate count, aggregate inspected count, aggregate skipped count, and aggregate inspection ratio.

## Typed archive load timing artifacts

Typed archive load timing artifacts are extracted from the debug report workload summary.

Artifacts:

```text
typed-archive-load-summary-<label>.csv
typed-archive-load-summary-<label>.txt
typed-archive-append-rebuild-summary-<label>.csv
typed-archive-append-rebuild-summary-<label>.txt
typed-archive-append-delta-query-summary-<label>.csv
typed-archive-append-delta-query-summary-<label>.txt
typed-archive-compaction-summary-<label>.csv
typed-archive-compaction-summary-<label>.txt
typed-archive-maintenance-summary-<label>.csv
typed-archive-maintenance-summary-<label>.txt
typed-archive-append-delta-maintenance-summary-<label>.csv
typed-archive-append-delta-maintenance-summary-<label>.txt
typed-query-index-archive-<label>.fse
```

The CSV preserves the workload-level relationship between querying the current in-memory typed query index, querying a typed query index loaded once before timing, and loading the `.fse` typed query index archive inside each measured iteration.

Required CSV fields include:

```text
dataset
max_depth
workload_name
matched_records
in_memory_elapsed
warm_loaded_elapsed
cold_loaded_elapsed
warm_loaded_to_in_memory_ratio
cold_loaded_to_in_memory_ratio
cold_loaded_to_warm_loaded_ratio
agreement
```

Every row should have:

```text
agreement = pass
```

The notes file records row count, agreement failures, total matched records, and mean timing ratios for warm-loaded and cold-loaded archive paths.

The `.fse` artifact is a persisted typed query index archive emitted by the benchmark run. It exists so the produced archive can be inspected as a benchmark artifact instead of only existing as a temporary timing file.

The benchmark output includes a validation status line after the emitted archive is reloaded and checked against in-memory typed query execution:

```text
Typed query index archive validated:
```

The review validator requires this status line in addition to requiring the archive file to exist and be non-empty.

## Typed archive append rebuild timing artifacts

Typed archive append rebuild timing artifacts are extracted from the debug report workload summary.

Artifacts:

```text
typed-archive-append-rebuild-summary-<label>.csv
typed-archive-append-rebuild-summary-<label>.txt
```

The CSV preserves the workload-level relationship between the source typed query index archive, the append batch, the rebuilt archive, and the measured append/rebuild/write path.

Required CSV fields include:

```text
dataset
max_depth
workload_name
base_record_count
appended_record_count
resulting_record_count
archive_bytes_before_append
archive_bytes_after_append
archive_byte_growth
matched_records_after_append
append_rebuild_elapsed
agreement
```

Every row should have:

```text
agreement = pass
```

The notes file records row count, agreement failures, total matched records after append, distinct record-count values, and distinct archive byte-growth values. The append rebuild elapsed field is the formatted duration emitted by the debug report.

## Typed archive append-delta query artifacts

Typed archive append-delta query artifacts are extracted from the debug report workload summary.

Artifacts:

```text
typed-archive-append-delta-query-summary-<label>.csv
typed-archive-append-delta-query-summary-<label>.txt
```

The CSV preserves the workload-level relationship between querying a base typed query index with a persisted append-delta batch and querying an equivalent rebuilt typed query index.

Required CSV fields include:

```text
dataset
max_depth
workload_name
base_record_count
appended_record_count
rebuilt_record_count
append_delta_matched_records
rebuilt_matched_records
append_delta_reconstructed_records
rebuilt_reconstructed_records
append_delta_candidate_ratio
rebuilt_candidate_ratio
rebuilt_to_append_delta_ratio
rebuilt_query_elapsed
append_delta_query_elapsed
agreement
```

Every row should have:

```text
agreement = pass
```

The append-delta and rebuilt paths should also report the same matched-record count. This artifact is the review evidence for querying inserts before a full hierarchy rebuild.

## Typed archive compaction and maintenance artifacts

Typed archive compaction and maintenance artifacts are extracted from debug report workload summaries.

Artifacts:

```text
typed-archive-compaction-summary-<label>.csv
typed-archive-compaction-summary-<label>.txt
typed-archive-maintenance-summary-<label>.csv
typed-archive-maintenance-summary-<label>.txt
typed-archive-append-delta-maintenance-summary-<label>.csv
typed-archive-append-delta-maintenance-summary-<label>.txt
```

The compaction summary records tombstone-driven archive rebuild evidence. The maintenance summaries record policy-selected archive actions and logical archive byte movement before and after maintenance.

Every row should have:

```text
agreement = pass
```

Use these artifacts to review Milestone H update, rebuild, and compaction behavior separately from owned-result query timing.

## Target workload artifacts

Target workload review artifacts are generated from repeated trial workload CSVs.

Artifacts:

```text
target-workload-trial-details-<label>.csv
target-workload-trial-summary-<label>.csv
target-workload-trial-notes-<label>.txt
```

When a previous target summary is available, the review workflow also generates:

```text
target-workload-trial-comparison-<label>.csv
target-workload-trial-comparison-<label>.txt
```

The target summary is focused on one configured boundary workload and groups rows by tree baseline.

Required summary fields include:

```text
baseline_name
workload_name
trial_count
mean_timing_ratio
mean_candidate_ratio
all_count_only_stats_match_owned
all_reference_stats_match_count_only
all_reusable_owned_stats_match_owned
```

The review manifest should mark target workload artifacts as `found` when target review runs and `skipped` when `-SkipTargetWorkloadReview` is used.

Review artifact validation enforces the target workload stats-equivalence fields when target artifacts are present:

```text
all_count_only_stats_match_owned
all_reference_stats_match_count_only
all_reusable_owned_stats_match_owned
```

When target comparison artifacts are required, validation also enforces the previous/current versions of those fields in the target comparison CSV.

## Target comparison validation scope

Target workload summary and target workload comparison artifacts are validated when present.

The full `-RequireValidatedComparisons` review flag requires the broader comparison set, not just target workload comparison artifacts. A target-only review smoke should provide previous target and materialization summaries without forcing unrelated aggregate, workload, or count-only comparisons.

## Materialization validation requirements

Materialization summary artifacts are not valid merely because they exist.

The review validator requires every row in:

  ```text
  materialization-mode-summary-<label>.csv
  ```

to report:

  ```text
  agreement = pass
  ```

When a materialization comparison artifact is present, every row in:

  ```text
  materialization-mode-summary-comparison-<label>.csv
  ```

must report:

  ```text
  agreement_status = pass
  ```

This protects the review workflow from accepting timing evidence when output contracts disagree on exact query semantics.
