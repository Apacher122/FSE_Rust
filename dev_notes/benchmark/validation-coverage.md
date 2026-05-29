# Benchmark Review Validation Coverage

This note records what the benchmark review validator checks and what each check is meant to protect.

The goal is to keep review validation focused on benchmark evidence quality. Validation should reject missing artifacts, malformed CSV contracts, invalid index state, and output-contract disagreement. It should not reject ordinary timing movement by itself.

## Validation scope

The review validator checks these artifact classes:

```text
core benchmark output
core benchmark CSVs
count-only summary artifacts
materialization summary artifacts
low-gap trial artifacts
target workload artifacts
optional comparison artifacts
review notes
review manifest
```

The validator is not a performance judge. It is a correctness and artifact-contract gate.

## Required core artifacts

These artifacts are always required by the low-gap review workflow:

```text
benchmark-output-<label>.txt
debug-output-<label>.txt
summary-<label>.csv
workloads-<label>.csv
count-only-workload-summary-<label>.csv
count-only-workload-summary-<label>.txt
materialization-mode-summary-<label>.csv
materialization-mode-summary-<label>.txt
low-selectivity-gap-<label>.csv
low-gap-regression-notes-<label>.txt
low-gap-trial-details-<label>.csv
low-gap-trial-summary-<label>.csv
low-gap-trial-notes-<label>.txt
low-gap-workload-trial-details-<label>.csv
low-gap-workload-trial-summary-<label>.csv
low-gap-review-notes-<label>.txt
low-gap-review-manifest-<label>.txt
```

The validator should fail if any required artifact is missing or empty.

## Core CSV validation

The review validator checks core CSV shape and index validity.

The summary CSV must include:

```text
index_valid
leaf_cardinality_valid
hierarchy_topology_valid
parent_child_bounds_valid
baseline_name
```

These fields must be true:

```text
index_valid
leaf_cardinality_valid
hierarchy_topology_valid
parent_child_bounds_valid
```

The workloads CSV must include:

```text
index_valid
leaf_cardinality_valid
baseline_name
workload_name
count_only_stats_match_owned
```

These fields must be true:

```text
index_valid
leaf_cardinality_valid
count_only_stats_match_owned
```

This protects the review from accepting benchmark evidence from an invalid index or from a workload where count-only semantics drifted from owned-result semantics.

## Count-only validation

The count-only workload summary must include:

```text
index_valid
leaf_cardinality_valid
baseline_name
selectivity_bucket
all_stats_match_owned
```

These fields must be true:

```text
index_valid
leaf_cardinality_valid
all_stats_match_owned
```

Count-only timing is review-grade evidence only when count-only structural stats match owned-result structural stats.

## Materialization validation

The materialization mode summary must include:

```text
dataset
max_depth
workload_name
matched_records
fresh_owned_elapsed
reusable_owned_elapsed
reference_elapsed
count_only_elapsed
agreement
```

Every row must have:

```text
agreement = pass
```

This protects the review from accepting materialization timing when owned, reusable-owned, reference-result, and count-only output contracts disagree.

## Target workload validation

Target workload artifacts are required unless target workload review is skipped.

When target workload review is enabled, the target summary must include:

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

These fields must be true:

```text
all_count_only_stats_match_owned
all_reference_stats_match_count_only
all_reusable_owned_stats_match_owned
```

This protects boundary workload timing from being accepted when output contracts disagree.

If `-SkipTargetWorkloadReview` is used, target workload artifacts should be marked skipped in the manifest rather than missing.

## Optional comparison validation

Comparison artifacts are optional unless the review command requires them.

When comparison artifacts are present, the validator checks the relevant CSV contracts.

### Aggregate trial comparison

The aggregate trial comparison CSV must include:

```text
baseline_name
previous_mean_low_mean_timing_ratio
current_mean_low_mean_timing_ratio
timing_classification
```

This comparison is a timing movement artifact. It does not enforce performance improvement.

### Workload trial comparison

The workload trial comparison CSV must include:

```text
baseline_name
previous_mean_weakest_timing_ratio
current_mean_weakest_timing_ratio
timing_classification
```

This comparison is a timing movement artifact. It does not enforce performance improvement.

### Count-only comparison

The count-only workload comparison CSV must include:

```text
baseline_name
selectivity_bucket
previous_weighted_count_only_speedup_ratio
current_weighted_count_only_speedup_ratio
weighted_count_only_speedup_classification
```

This comparison is a review signal for count-only movement by selectivity bucket.

### Materialization comparison

The materialization mode comparison CSV must include:

```text
dataset
max_depth
workload_name
matched_records_delta
fresh_owned_elapsed_classification
reusable_owned_elapsed_classification
reference_elapsed_classification
count_only_elapsed_classification
agreement_status
```

Every row must have:

```text
agreement_status = pass
```

Timing classifications may be `improved`, `stable/noise`, `regressed`, or a similar review category. Timing movement alone is not a validation failure.

### Target workload comparison

The target workload comparison CSV must include:

  ```text
  baseline_name
  previous_workload_name
  current_workload_name
  previous_mean_timing_ratio
  current_mean_timing_ratio
  timing_classification
  previous_all_count_only_stats_match_owned
  current_all_count_only_stats_match_owned
  previous_all_reference_stats_match_count_only
  current_all_reference_stats_match_count_only
  previous_all_reusable_owned_stats_match_owned
  current_all_reusable_owned_stats_match_owned
  ```

These fields must be true:

  ```text
  previous_all_count_only_stats_match_owned
  current_all_count_only_stats_match_owned
  previous_all_reference_stats_match_count_only
  current_all_reference_stats_match_count_only
  previous_all_reusable_owned_stats_match_owned
  current_all_reusable_owned_stats_match_owned
  ```

Target timing movement is not a validation failure by itself.

## Require comparisons flag

`-RequireValidatedComparisons` is for a full previous/current comparison review.

It requires the full comparison set:

  ```text
  single-run low-selectivity comparison
  aggregate trial comparison
  workload trial comparison
  count-only workload comparison
  materialization mode comparison
  target workload comparison
  ```

Use it when the previous run was preserved as a complete review baseline, usually through `-CopyArtifactsToRunFolder` and `-PreviousLabel`.

Target workload comparison and materialization comparison are also validated whenever their artifacts are present, even when `-RequireValidatedComparisons` is not used.

Do not use `-RequireValidatedComparisons` for a target-only or materialization-only smoke unless the full previous comparison input set is also provided.

## What validation should not do

Validation should not fail because:

  ```text
  a timing result regressed
  a timing result improved
  a timing result is classified as stable/noise
  a comparison artifact is skipped because no previous input was supplied
  target workload review was explicitly skipped
  ```

Validation should fail because:

  ```text
  a required artifact is missing
  a required CSV column is missing
  an index validity field is false
  a stats-equivalence field is false
  a materialization agreement field is not pass
  a target workload agreement field is false
  a comparison is required but skipped
  ```

## Review rule

Use validation as a gate for evidence quality.

Use artifact review to decide keep, fix, revert, or gather more repeated-trial evidence.
