# Count-Only Benchmark Workflow

This file documents the count-only query benchmark workflow for the FSE Rust implementation.

Count-only execution is a separate output contract from owned-result execution.

Owned-result execution answers:

```text
which records matched, returned as materialized Vec<Vector>
```

Count-only execution answers:

```text
how many records matched, returned as an exact cardinality
```

Do not mix owned-result benchmark conclusions with count-only benchmark conclusions.

## Public count-only API

The public count-only query APIs are:

```text
count_query_matches
count_query_matches_with_stats
```

The count-only API preserves the same query semantics as owned-result execution:

```text
geometric pruning
retained-leaf selection
exact predicate evaluation
matched record counting
```

The difference is output representation.

Owned-result query execution returns:

```text
Vec<Vector>
```

Count-only query execution returns:

```text
usize
```

or:

```text
QueryCountReport
```

when statistics are requested.

## Output contract distinction

Owned-result query:

```text
returns materialized rows
pays owned Vector construction cost
should be used when the caller needs returned records
```

Count-only query:

```text
returns exact cardinality
does not materialize owned Vector results
should be used when the caller only needs a count
```

This distinction is important because result materialization can dominate runtime when many rows match.

## Resultless retained diagnostic versus public count-only timing

The benchmark system contains two different count-like measurements.

The first is the debug-only resultless retained diagnostic:

```text
resultless retained diagnostic:
  measures retained-leaf match counting after traversal is already available
  does not include root classification
  does not include traversal
  does not include public query report construction
  debug-only
  not a public query API
```

The second is the public count-only query timing:

```text
count-only query timing:
  includes root classification
  includes traversal
  includes retained-leaf exact evaluation
  includes stats/report construction
  public query API
```

Owned-result query timing includes:

```text
root classification
traversal
retained-leaf exact evaluation
owned Vector result materialization
```

Interpretation:

```text
resultless retained timing explains internal retained-leaf counting cost
count-only query timing measures the real public cardinality API
owned-result query timing measures materialized row-return behavior
```

## Result ownership timing conclusion

Resultless target workload diagnostics showed that target workload cost is mostly result materialization work, not query pruning or predicate checking.

The small target workload result was:

```text
dataset: small
target workload: cluster_boundary_range
candidate records: 10
matched records: 5
covered leaves: 0
partial leaves: 2
covered records: 0
partial records: 10
average resultless retained elapsed: about 30ns
average retained execution elapsed: about 161ns
estimated result ownership overhead: about 131ns
estimated resultless retained share: about 19%
```

The large target workload result was:

```text
dataset: large
target workload: large_cross_cluster_boundary
candidate records: 78
matched records: 71
covered leaves: 8
partial leaves: 2
covered records: 62
partial records: 16
average resultless retained elapsed: about 38ns
average retained execution elapsed: about 1.65us
estimated result ownership overhead: about 1.61us
estimated resultless retained share: about 2%
```

The useful conclusion is:

```text
target workload traversal is not the dominant cost
predicate checking is not the dominant cost
plain result Vec allocation is small
owned Vector materialization dominates retained execution when many rows match
```

This means future performance work should not be framed as another boundary traversal fix unless new evidence changes the diagnosis.

## Count-only target timing baseline

The count-only target comparison measures the public count-only API against the owned-result API.

This comparison confirms whether the real public count-only path is faster, not merely whether an internal diagnostic helper is faster.

The small target workload baseline is:

```text
dataset: small
target workload: cluster_boundary_range
owned average elapsed: about 249ns
count-only average elapsed: about 103ns
estimated owned result overhead: about 146ns
count-only speedup: about 2.42x
owned matched records: 5
count-only matched records: 5
matched records agree: true
owned candidate records: 10
count-only candidate records: 10
owned retained leaves: 2
count-only retained leaves: 2
owned visited nodes: 13
count-only visited nodes: 13
```

The large target workload baseline is:

```text
dataset: large
target workload: large_cross_cluster_boundary
owned average elapsed: about 2.311us
count-only average elapsed: about 389ns
estimated owned result overhead: about 1.922us
count-only speedup: about 5.94x
owned matched records: 71
count-only matched records: 71
matched records agree: true
owned candidate records: 78
count-only candidate records: 78
owned retained leaves: 10
count-only retained leaves: 10
owned visited nodes: 45
count-only visited nodes: 45
```

The current interpretation is:

```text
count-only execution confirms that pruning and exact match counting are not the dominant target cost
owned Vector materialization is the dominant target cost when many rows match
the count-only API is a valid separate query mode, not a replacement for owned-result execution
```

Do not compare count-only timing against owned-result timing as if they answer the same application question.

They represent different output contracts:

```text
use owned-result timing when the caller needs materialized rows
use count-only timing when the caller only needs cardinality
```

## Count-only metrics in workload CSV

The workload CSV includes count-only metrics for each workload row.

The added fields are:

```text
count_only_visited_nodes
count_only_retained_leaves
count_only_reconstructed_records
count_only_matched_records
count_only_average_elapsed_ns
estimated_owned_result_overhead_ns
count_only_speedup_ratio
count_only_stats_match_owned
```

The important correctness field is:

```text
count_only_stats_match_owned
```

This should be:

```text
true
```

for every row before using count-only timing as evidence.

Meaning:

```text
count_only_stats_match_owned = true:
  count-only execution and owned-result execution agree on structural query work

count_only_stats_match_owned = false:
  do not use count-only timing until the mismatch is explained
```

## Count-only summary artifact workflow

The benchmark review workflow writes count-only summary artifacts alongside the normal owned-result benchmark artifacts.

The count-only summary artifacts are:

```text
count-only-workload-summary-<label>.csv
count-only-workload-summary-<label>.txt
```

The CSV artifact is the machine-readable summary.

It groups count-only behavior by:

```text
baseline
selectivity bucket
```

The notes artifact is the human-readable review summary.

It is meant for quick inspection during commit review.

## Count-only summary CSV fields

The count-only summary CSV includes these core fields:

```text
dataset_records
timing_iterations
max_depth
index_valid
leaf_cardinality_valid
baseline_name
selectivity_bucket
workload_count
stats_agree_count
all_stats_match_owned
total_fse_reconstructed_records
total_count_only_reconstructed_records
total_fse_matched_records
total_count_only_matched_records
mean_candidate_ratio
mean_owned_average_elapsed_ns
mean_count_only_average_elapsed_ns
mean_owned_result_overhead_ns
mean_count_only_speedup_ratio
weighted_count_only_speedup_ratio
total_owned_average_elapsed_ns
total_count_only_average_elapsed_ns
total_owned_result_overhead_ns
```

The most important correctness field is:

```text
all_stats_match_owned
```

This field must be true before count-only timing is used as evidence.

Interpretation rule:

```text
all_stats_match_owned = true:
  count-only and owned-result execution agree on structural query work

all_stats_match_owned = false:
  do not use count-only timing for performance conclusions until the mismatch is explained
```

## Count-only summary notes file

The count-only summary notes file includes:

```text
dataset records
timing iterations
max depth
index validation state
leaf cardinality validation state
stats agreement status
low-selectivity count-only speedup by baseline
best low-selectivity count-only speedup
full-selectivity behavior note
```

The notes file should include a section like:

```text
Stats agreement
---------------
all count-only summary rows match owned-result structural stats: true
```

If the stats agreement is false, stop and investigate before using the count-only timing numbers.

## Low-selectivity count-only interpretation

Low-selectivity rows are usually the most useful count-only comparison rows because they preserve selective query behavior while removing owned result construction.

Use low-selectivity rows to understand:

```text
count-only query mode movement
cardinality-only workload behavior
whether count-only speedup persists across labels
whether count-only elapsed time is improving or regressing
```

Low-selectivity count-only timing should not be used to claim owned-result query speedups.

## Full-selectivity count-only caution

Full-selectivity count-only speedups can be extremely large.

Reason:

```text
count-only root coverage can return total cardinality without materializing all owned Vector results
```

This is expected and useful for cardinality-only workloads.

Do not interpret full-selectivity count-only speedup as a general traversal improvement.

Full-selectivity count-only rows should be reviewed separately from low-selectivity rows.

## Count-only comparison artifact workflow

The review workflow can compare count-only summary artifacts across labels.

The count-only comparison artifacts are:

```text
count-only-workload-summary-comparison-<label>.csv
count-only-workload-summary-comparison-<label>.txt
```

These artifacts are generated when the current review can find a previous count-only workload summary.

The previous summary is resolved through the normal previous-label lookup rules:

```text
1. explicit previous count-only summary path, when provided
2. flat OutputDir artifact path
3. organized run folder at OutputDir/runs/<previous-label>/
```

Example review command:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "current-review" `
  -PreviousLabel "previous-review" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000
```

## Count-only comparison CSV fields

The comparison CSV is machine-readable.

It compares rows by:

```text
baseline_name
selectivity_bucket
```

The comparison CSV includes:

```text
previous_workload_count
current_workload_count
previous_all_stats_match_owned
current_all_stats_match_owned
previous_weighted_count_only_speedup_ratio
current_weighted_count_only_speedup_ratio
weighted_count_only_speedup_delta
weighted_count_only_speedup_classification
previous_mean_count_only_average_elapsed_ns
current_mean_count_only_average_elapsed_ns
mean_count_only_average_elapsed_delta_ns
mean_count_only_average_elapsed_classification
previous_mean_owned_average_elapsed_ns
current_mean_owned_average_elapsed_ns
mean_owned_average_elapsed_delta_ns
previous_mean_owned_result_overhead_ns
current_mean_owned_result_overhead_ns
mean_owned_result_overhead_delta_ns
```

## Count-only comparison notes file

The comparison notes file is the human-readable review artifact.

It includes:

```text
row coverage
stats agreement
low-selectivity speedup movement
per-baseline low-selectivity movement
interpretation guidance
```

The most important correctness condition is:

```text
all compared rows match owned-result structural stats in both summaries: true
```

If this is false, do not use count-only timing movement as performance evidence until the mismatch is explained.

## Count-only comparison interpretation

Count-only comparison classifications are separate from owned-result benchmark classifications.

Interpretation rules:

```text
weighted count-only speedup classification:
  higher is better

mean count-only elapsed classification:
  lower is better

owned-result low-gap classification:
  separate benchmark mode
  do not mix with count-only movement
```

A count-only comparison can improve while owned-result timing regresses.

Owned-result timing can improve while count-only movement is stable.

These are different output contracts and should be reviewed separately.

Use count-only comparison artifacts for:

```text
count-only query mode movement
cardinality-only workload evaluation
result materialization overhead tracking
checking whether count-only improvements persist across labels
```

Do not use count-only comparison artifacts for:

```text
claiming owned-result query speedups
claiming materialized row-return performance improved
accepting traversal changes without owned-result benchmark evidence
```

## Count-only comparison acceptance rule

Count-only comparison artifacts are acceptable when:

```text
benchmark validation passes
leaf cardinality validation passes
comparison row coverage is complete
missing previous rows is 0
missing current rows is 0
previous_all_stats_match_owned is true
current_all_stats_match_owned is true
low-selectivity rows are reviewed separately from full-selectivity rows
count-only conclusions remain separate from owned-result conclusions
```

## Recommended review order

When count-only comparison artifacts exist, use this order:

```text
1. Check normal benchmark validation.
2. Check owned-result low-gap and target workload artifacts.
3. Check count-only workload summary notes.
4. Check count-only comparison notes.
5. Use count-only comparison CSV only when row-level machine-readable detail is needed.
```

## Recommended future benchmark categories

Future benchmark reporting should keep these modes separate:

```text
owned-result benchmark:
  measures full query materialization cost

count-only benchmark:
  measures exact cardinality cost without owned result materialization

diagnostic retained benchmark:
  measures internal retained-leaf work and should remain debug-only
```

## Current benchmark interpretation

The current benchmark interpretation is:

```text
owned-result review answers materialized query performance
count-only review answers exact-cardinality query performance
count-only comparison artifacts track count-only movement across labels
owned Vector materialization remains part of owned-result query cost
```

The count-only baseline supports this design direction:

```text
FSE should expose separate query modes for separate output contracts
result materialization should not be treated as unavoidable query-evaluation cost when the caller only needs a count
```
