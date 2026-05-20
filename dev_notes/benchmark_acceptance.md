# Benchmark Acceptance Notes

This note defines how benchmark artifacts should be interpreted before accepting query-performance changes.

The current benchmark target is the small clustered 2D workload. The most useful performance signal is the KD-tree low-selectivity gap because it is the closest tree-baseline comparison.

## Required checks before accepting a performance commit

Run the normal Rust checks:

```powershell
cargo fmt
cargo test
cargo check --release
```

Run the normal benchmark artifact bundle:

```powershell
.\scripts\run-small-benchmark.ps1 `
  -Label "<commit-label>" `
  -Iterations 10000 `
  -PreviousLowSelectivityGapCsv "benchmark_artifacts\low-selectivity-gap-<previous-label>.csv"
```

Run repeated low-gap trials:

```powershell
.\scripts\run-low-gap-trials.ps1 `
  -Label "<commit-label>" `
  -Trials 5 `
  -Iterations 10000 `
  -PreviousLowSelectivityGapCsv "benchmark_artifacts\low-selectivity-gap-<previous-label>.csv"
```

Compare repeated aggregate low-gap summaries when a previous trial summary exists:

```powershell
.\scripts\compare-low-gap-trial-summary.ps1 `
  -Label "<commit-label>" `
  -PreviousTrialSummaryCsv "benchmark_artifacts\low-gap-trial-summary-<previous-label>.csv" `
  -CurrentTrialSummaryCsv "benchmark_artifacts\low-gap-trial-summary-<commit-label>.csv"
```

Compare repeated weakest-workload summaries when a previous workload-trial summary exists:

```powershell
.\scripts\compare-low-gap-workload-summary.ps1 `
  -Label "<commit-label>" `
  -PreviousWorkloadSummaryCsv "benchmark_artifacts\low-gap-workload-trial-summary-<previous-label>.csv" `
  -CurrentWorkloadSummaryCsv "benchmark_artifacts\low-gap-workload-trial-summary-<commit-label>.csv"
```

The repeated-trial summary is the primary decision artifact for low-gap performance commits.

For boundary-focused commits, the workload-trial comparison is the primary decision artifact for the weakest workload.

## Required benchmark artifacts for performance review

A complete performance-review bundle should include:

```text
benchmark-output-<commit-label>.txt
debug-output-<commit-label>.txt
summary-<commit-label>.csv
workloads-<commit-label>.csv
low-selectivity-gap-<commit-label>.csv
low-gap-regression-notes-<commit-label>.txt
low-gap-trial-details-<commit-label>.csv
low-gap-trial-summary-<commit-label>.csv
low-gap-trial-notes-<commit-label>.txt
low-gap-workload-trial-details-<commit-label>.csv
low-gap-workload-trial-summary-<commit-label>.csv
low-gap-trial-comparison-<commit-label>.csv
low-gap-trial-comparison-<commit-label>.txt
low-gap-workload-trial-comparison-<commit-label>.csv
low-gap-workload-trial-comparison-<commit-label>.txt
```

Reporting-only commits do not always need the full performance-review bundle. Query-performance commits should produce it before being accepted.

## Primary aggregate acceptance signal

Use:

```text
low-gap-trial-summary-<commit-label>.csv
```

The primary field is:

```text
kd_tree.mean_low_mean_timing_ratio
```

The comparison field is:

```text
kd_tree.delta_from_previous
```

The default noise threshold is:

```text
+/- 0.030000
```

A positive delta means FSE improved relative to KD-tree because the timing ratio is:

```text
baseline elapsed / FSE elapsed
```

Higher is better for FSE.

## Aggregate trial comparison signal

Use:

```text
low-gap-trial-comparison-<commit-label>.csv
low-gap-trial-comparison-<commit-label>.txt
```

The primary aggregate comparison fields are:

```text
kd_tree.previous_mean_low_mean_timing_ratio
kd_tree.current_mean_low_mean_timing_ratio
kd_tree.mean_low_mean_timing_delta
kd_tree.timing_classification
kd_tree.previous_range_low_mean_timing_ratio
kd_tree.current_range_low_mean_timing_ratio
kd_tree.range_low_mean_timing_delta
kd_tree.previous_sample_stddev_low_mean_timing_ratio
kd_tree.current_sample_stddev_low_mean_timing_ratio
kd_tree.sample_stddev_low_mean_timing_delta
kd_tree.mean_low_weighted_candidate_delta
kd_tree.mean_low_weighted_avoidance_delta
```

A useful aggregate low-gap improvement should look like this:

```text
mean low timing delta: greater than +0.030000
classification: improved
mean weighted candidate delta: 0.000000 unless intentionally changed
mean weighted avoidance delta: 0.000000 unless intentionally changed
range and sample stddev do not grow enough to make the apparent improvement suspicious
```

A noisy aggregate result looks like this:

```text
classification: stable/noise
mean low timing delta: between -0.030000 and +0.030000
```

A regressed aggregate result looks like this:

```text
classification: regressed
mean low timing delta: less than -0.030000
```

## Boundary workload acceptance signal

Use:

```text
low-gap-workload-trial-summary-<commit-label>.csv
```

The primary boundary fields are:

```text
kd_tree.most_frequent_weakest_workload
kd_tree.mean_weakest_timing_ratio
kd_tree.mean_baseline_average_elapsed_ns
kd_tree.mean_fse_average_elapsed_ns
kd_tree.mean_fse_visited_nodes
kd_tree.mean_fse_retained_leaves
kd_tree.mean_fse_reconstructed_records
kd_tree.mean_baseline_evaluated_records
kd_tree.mean_candidate_ratio
kd_tree.mean_nodes_per_record
```

When comparing two workload-trial summaries, use:

```text
low-gap-workload-trial-comparison-<commit-label>.csv
low-gap-workload-trial-comparison-<commit-label>.txt
```

The primary workload comparison fields are:

```text
kd_tree.previous_weakest_workload
kd_tree.current_weakest_workload
kd_tree.previous_mean_weakest_timing_ratio
kd_tree.current_mean_weakest_timing_ratio
kd_tree.mean_weakest_timing_delta
kd_tree.timing_classification
kd_tree.mean_fse_visited_nodes_delta
kd_tree.mean_fse_reconstructed_records_delta
kd_tree.mean_candidate_ratio_delta
kd_tree.mean_nodes_per_record_delta
```

The expected current KD-tree weak point is:

```text
cluster_boundary_range
```

A boundary-focused performance commit should improve the repeated-trial weakest workload mean, not just one single debug run.

If a previous workload-trial summary exists, compare:

```text
previous kd_tree.mean_weakest_timing_ratio
current kd_tree.mean_weakest_timing_ratio
```

A meaningful boundary improvement should move the repeated-trial mean by at least:

```text
+0.030000
```

The boundary workload summary is also used to confirm that the same workload is actually being targeted. If `cluster_boundary_range` is not the most frequent weakest KD-tree workload, do not assume a boundary optimization was tested cleanly.

## Accept rule

Accept a query-performance commit when all of these are true:

```text
cargo test passes
cargo check --release passes
matched record counts remain exact
validation remains pass
candidate counts are unchanged unless the commit intentionally changes pruning
flat_scan and r_tree do not show broad weighted-timing regressions
```

Then require at least one of these aggregate performance signals:

```text
kd_tree.mean_low_mean_timing_delta >= +0.030000
kd_tree.timing_classification == improved
```

or, for a boundary-focused optimization, require this comparison signal:

```text
kd_tree.current_weakest_workload == cluster_boundary_range
kd_tree.timing_classification == improved
kd_tree.mean_weakest_timing_delta >= +0.030000
kd_tree.mean_fse_visited_nodes_delta <= 0 unless intentionally changed
kd_tree.mean_fse_reconstructed_records_delta <= 0 unless intentionally changed
kd_tree.mean_candidate_ratio_delta <= 0 unless intentionally changed
```

A performance commit should also preserve the expected structural invariants unless the commit intentionally changes traversal or index construction:

```text
same dataset record count
same workload count
same selected baselines
same validation status
same exact result correctness
```

## Rerun rule

Rerun the benchmark before deciding when any of these are true:

```text
kd_tree.classification == stable/noise
abs(kd_tree.delta_from_previous) < 0.030000
kd_tree.timing_classification == stable/noise
abs(kd_tree.mean_low_mean_timing_delta) < 0.030000
abs(kd_tree.mean_weakest_timing_delta) < 0.030000
kd_tree.range_low_mean_timing_ratio is larger than 0.030000
kd_tree.range_weakest_timing_ratio is larger than 0.030000
debug output and CSV output disagree on the weakest workload
one trial is much better or worse than the others
cluster_boundary_range is not consistently the weakest KD-tree workload
aggregate trial comparison and workload trial comparison disagree
```

A single good benchmark run is not enough to keep a query-performance commit if the repeated-trial mean stays inside the noise band.

A single good boundary workload row is also not enough. Use the workload trial summary and the workload comparison artifact before deciding.

## Revert rule

Revert a query-performance commit when any of these are true:

```text
cargo test fails
cargo check --release fails
matched record counts change
validation fails
candidate counts change unexpectedly
kd_tree.mean_low_mean_timing_delta <= -0.030000
kd_tree.timing_classification == regressed
kd_tree.mean_weakest_timing_delta <= -0.030000
the code becomes more specialized without a repeated-trial win
```

If a commit only changes report formatting, artifact generation, or notes, do not revert it based on timing movement alone. Reporting-only commits should be judged by artifact correctness.

## Current diagnostic target

The current KD-tree weak point is usually:

```text
cluster_boundary_range
```

The pressure pattern is:

```text
KD-tree baseline records: 9
FSE visited nodes: 13
FSE retained leaves: 2
FSE reconstructed records: 10
```

The repeated workload-trial summary from Commit 160 confirmed that `cluster_boundary_range` was the weakest KD-tree workload in all five trials.

This means future optimization attempts should focus on boundary-workload pressure, not sibling overlap.

## Full performance-review workflow

Use this workflow for any query-performance commit.

First, run checks:

```powershell
cargo fmt
cargo test
cargo check --release
```

Second, run the normal artifact bundle:

```powershell
.\scripts\run-small-benchmark.ps1 `
  -Label "<commit-label>" `
  -Iterations 10000 `
  -PreviousLowSelectivityGapCsv "benchmark_artifacts\low-selectivity-gap-<previous-label>.csv"
```

Third, run repeated low-gap trials:

```powershell
.\scripts\run-low-gap-trials.ps1 `
  -Label "<commit-label>" `
  -Trials 5 `
  -Iterations 10000 `
  -PreviousLowSelectivityGapCsv "benchmark_artifacts\low-selectivity-gap-<previous-label>.csv"
```

Fourth, compare aggregate low-gap trial summaries:

```powershell
.\scripts\compare-low-gap-trial-summary.ps1 `
  -Label "<commit-label>" `
  -PreviousTrialSummaryCsv "benchmark_artifacts\low-gap-trial-summary-<previous-label>.csv" `
  -CurrentTrialSummaryCsv "benchmark_artifacts\low-gap-trial-summary-<commit-label>.csv"
```

Fifth, compare weakest-workload trial summaries:

```powershell
.\scripts\compare-low-gap-workload-summary.ps1 `
  -Label "<commit-label>" `
  -PreviousWorkloadSummaryCsv "benchmark_artifacts\low-gap-workload-trial-summary-<previous-label>.csv" `
  -CurrentWorkloadSummaryCsv "benchmark_artifacts\low-gap-workload-trial-summary-<commit-label>.csv"
```

## Workload comparison workflow

Use the workload comparator after producing repeated-trial artifacts for a new commit:

```powershell
.\scripts\compare-low-gap-workload-summary.ps1 `
  -Label "<commit-label>" `
  -PreviousWorkloadSummaryCsv "benchmark_artifacts\low-gap-workload-trial-summary-<previous-label>.csv" `
  -CurrentWorkloadSummaryCsv "benchmark_artifacts\low-gap-workload-trial-summary-<commit-label>.csv"
```

Expected outputs:

```text
low-gap-workload-trial-comparison-<commit-label>.csv
low-gap-workload-trial-comparison-<commit-label>.txt
```

Use the comparison notes to check:

```text
previous weakest workload
current weakest workload
mean weakest timing delta
classification
mean FSE visited nodes delta
mean FSE reconstructed records delta
mean candidate ratio delta
mean nodes per record delta
```

For a boundary-focused optimization, a clean positive result should look like this:

```text
current weakest workload: cluster_boundary_range
mean weakest timing delta: greater than +0.030000
classification: improved
mean FSE visited nodes delta: 0.000000 or lower
mean FSE reconstructed records delta: 0.000000 or lower
mean candidate ratio delta: 0.000000 or lower
```

A noisy result looks like this:

```text
classification: stable/noise
mean weakest timing delta: between -0.030000 and +0.030000
```

A weak or suspicious result looks like this:

```text
current weakest workload changed away from cluster_boundary_range
timing improved but visited nodes increased
timing improved but reconstructed records increased
timing improved but candidate ratio increased
```

## Aggregate comparison workflow

Use the aggregate comparator after producing repeated-trial artifacts for a new commit:

```powershell
.\scripts\compare-low-gap-trial-summary.ps1 `
  -Label "<commit-label>" `
  -PreviousTrialSummaryCsv "benchmark_artifacts\low-gap-trial-summary-<previous-label>.csv" `
  -CurrentTrialSummaryCsv "benchmark_artifacts\low-gap-trial-summary-<commit-label>.csv"
```

Expected outputs:

```text
low-gap-trial-comparison-<commit-label>.csv
low-gap-trial-comparison-<commit-label>.txt
```

Use the comparison notes to check:

```text
previous mean low timing
current mean low timing
mean low timing delta
classification
range delta
sample stddev delta
mean weighted candidate delta
mean weighted avoidance delta
```

For an aggregate low-selectivity optimization, a clean positive result should look like this:

```text
mean low timing delta: greater than +0.030000
classification: improved
mean weighted candidate delta: 0.000000 or lower unless intentionally changed
mean weighted avoidance delta: 0.000000 or higher unless intentionally changed
range delta: not large enough to explain the improvement as noise
sample stddev delta: not large enough to explain the improvement as noise
```

A noisy result looks like this:

```text
classification: stable/noise
mean low timing delta: between -0.030000 and +0.030000
```

A weak or suspicious result looks like this:

```text
timing improved but candidate ratio increased
timing improved but avoidance ratio decreased
timing improved but range or sample stddev grew sharply
```

## Interpreting low-gap trial notes

Use this file:

```text
low-gap-trial-notes-<commit-label>.txt
```

A useful aggregate performance win should look like this:

```text
kd_tree
-------
classification: improved
delta from previous: greater than +0.030000
range: not much larger than the improvement
mean weighted candidate: unchanged unless intended
mean weighted avoidance: unchanged unless intended
```

A useful boundary workload win should look like this:

```text
kd_tree
-------
most frequent weakest workload: cluster_boundary_range
mean weakest timing: improved by more than +0.030000 against the previous workload-trial summary
mean FSE visited nodes: unchanged or lower
mean FSE reconstructed records: unchanged or lower
mean baseline evaluated records: unchanged
mean candidate ratio: unchanged or lower
```

A noisy result looks like this:

```text
classification: stable/noise
delta from previous: between -0.030000 and +0.030000
range or sample stddev is close to the size of the apparent improvement
```

A regression looks like this:

```text
classification: regressed
delta from previous: less than -0.030000
boundary workload mean timing regressed by more than -0.030000
```

## Decision order

Use this order when reviewing benchmark artifacts:

```text
1. cargo test and cargo check result
2. normal compact benchmark summary
3. low-gap trial summary CSV
4. low-gap trial comparison CSV and notes
5. low-gap workload trial summary CSV
6. low-gap workload comparison CSV and notes
7. low-gap trial notes
8. workload CSV
9. debug report
10. single-run low-gap regression notes
```

The single-run low-gap regression notes are useful, but they are not the final decision source for performance commits. The repeated-trial summary and comparator outputs are stronger.

For boundary-focused commits, the workload trial summary and workload comparison artifact are required because the aggregate low-selectivity bucket can hide whether `cluster_boundary_range` actually improved.
