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
  runs/
    <label>/
      benchmark-output-<label>.txt
      debug-output-<label>.txt
      summary-<label>.csv
      workloads-<label>.csv
      count-only-workload-summary-<label>.csv
      count-only-workload-summary-<label>.txt
      low-gap-review-notes-<label>.txt
      low-gap-review-manifest-<label>.txt
      low-gap-trials-<label>/
```

The flat root is useful for active comparison workflows.

The organized run folder is useful for long-term inspection and cleanup.

## Benchmark artifact organization workflow

Benchmark reviews can produce many artifacts per run. The flat `benchmark_artifacts` directory is useful for active comparison workflows, but it becomes difficult to inspect once many labels have accumulated.

The artifact organization helper is:

```text
scripts/organize-benchmark-artifacts.ps1
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
.\scripts\organize-benchmark-artifacts.ps1 -DryRun
```

The recommended normal use is copy mode:

```powershell
.\scripts\organize-benchmark-artifacts.ps1 -Copy
```

Copy mode creates organized folders while preserving the flat artifact root. This is the safest mode when a label is still being actively inspected or used as a comparison baseline.

Use move mode only when old artifacts no longer need to remain in the flat root:

```powershell
.\scripts\organize-benchmark-artifacts.ps1
```

Organize a single label with:

```powershell
.\scripts\organize-benchmark-artifacts.ps1 `
  -Label "<label>" `
  -Copy
```

Use `-Force` only when intentionally overwriting already-organized artifacts:

```powershell
.\scripts\organize-benchmark-artifacts.ps1 `
  -Copy `
  -Force
```

The organization script recognizes normal benchmark artifacts, low-gap artifacts, target workload artifacts, count-only artifacts, review manifests, review notes, and repeated-trial folders.

Unrecognized files are left in place. This prevents accidental movement of manually-created notes, temporary files, or unrelated data.

## Organized previous-label resolution

The review runner can resolve previous-label artifacts from both the flat artifact directory and organized run folders.

When a review command uses:

```powershell
.\scripts\run-low-gap-review.ps1 `
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
.\scripts\organize-benchmark-artifacts.ps1 `
  -Label "previous-review" `
  -Copy `
  -Force
```

Example fallback review command:

```powershell
.\scripts\run-low-gap-review.ps1 `
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
.\scripts\run-low-gap-review.ps1 `
  -Label "<label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000
```

Organized-copy review command:

```powershell
.\scripts\run-low-gap-review.ps1 `
  -Label "<label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000 `
  -CopyArtifactsToRunFolder
```

If re-running the same label and intentionally overwriting the organized copy, use:

```powershell
.\scripts\run-low-gap-review.ps1 `
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

## Flat benchmark artifact cleanup workflow

The flat `benchmark_artifacts` directory is useful for active development, but it can become crowded after repeated review runs.

The cleanup helper is:

```text
scripts/cleanup-flat-benchmark-artifacts.ps1
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
.\scripts\cleanup-flat-benchmark-artifacts.ps1
```

Preview cleanup for one label:

```powershell
.\scripts\cleanup-flat-benchmark-artifacts.ps1 `
  -Label "my-review"
```

Delete eligible flat artifacts for one label:

```powershell
.\scripts\cleanup-flat-benchmark-artifacts.ps1 `
  -Label "my-review" `
  -Delete `
  -Force
```

Delete all eligible flat artifacts that already have organized copies:

```powershell
.\scripts\cleanup-flat-benchmark-artifacts.ps1 `
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
.\scripts\organize-benchmark-artifacts.ps1 -Copy

.\scripts\cleanup-flat-benchmark-artifacts.ps1

.\scripts\cleanup-flat-benchmark-artifacts.ps1 `
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
repeated-trial folders
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

The debug report includes a workload materialization mode summary. The benchmark artifact workflow should keep a machine-readable copy of that section so owned-result, reusable-owned, reference-result, and count-only timings can be reviewed without manually scraping terminal output.

The summary artifact is generated from the debug report:

```powershell
.\scripts\summarize-materialization-mode-summary.ps1 `
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
count_only_elapsed
count_speedup
reference_speedup
reusable_speedup
agreement
```

The agreement column must remain `pass` for every workload before using materialization timing as evidence.

## Materialization summary artifacts

Materialization-mode summaries are extracted from benchmark debug output by:

```text
scripts/summarize-materialization-mode-summary.ps1
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
count-only
```

It also stores the dataset, effective max depth, matched record count, speedup ratios, and an agreement column.

Use the comparison helper when comparing two materialization summary CSVs:

```powershell
.\scripts\compare-materialization-mode-summary.ps1 `
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
