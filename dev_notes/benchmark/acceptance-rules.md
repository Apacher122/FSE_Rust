# Benchmark Acceptance Rules

This file defines benchmark acceptance rules for the FSE Rust implementation.

These rules apply to benchmark, query-performance, diagnostic, and reporting commits unless a commit explicitly documents a different acceptance standard.

## Current benchmark phase

The project is currently in the benchmarking, diagnostic tooling, and measurement-discipline phase.

The benchmark workflow compares FSE against:

```text
flat_scan
kd_tree
r_tree
```

The benchmark workflow currently distinguishes these FSE output contracts:

```text
owned-result query execution:
  returns materialized Vec<Vector> results

reference-result query execution:
  returns exact row references in a Vec<QueryResultReference>

reference-visitor query execution:
  visits exact row references without constructing the reference Vec

count-only query execution:
  returns exact cardinality without owned Vector materialization

existence query execution:
  returns whether the exact result set is non-empty
```

Owned-result, reference-result, reference-visitor, count-only, and existence conclusions are interpreted separately because each output contract measures a different returned value over the same exact query semantics.

## General acceptance requirements

A commit is acceptable only when the relevant checks pass:

```text
cargo fmt
cargo test
cargo check --release
```

For benchmark-facing commits, benchmark validation must also pass:

```text
Validation: pass
index_valid: true
leaf_cardinality_valid: true
hierarchy_topology_valid: true
parent_child_bounds_valid: true
```

For large dataset reviews, the expected valid baseline is:

```text
dataset: large
max depth: 16
leaf_cardinality_valid: true
```

For small dataset reviews, the expected valid baseline is:

```text
dataset: small
max depth: 8
leaf_cardinality_valid: true
```

## Timing ratio interpretation

Benchmark timing ratio is interpreted as:

```text
baseline elapsed / FSE elapsed
```

Higher is better for FSE.

Examples:

```text
timing ratio > 1.0:
  FSE is faster than the baseline

timing ratio = 1.0:
  FSE and the baseline are equal within the measured timing

timing ratio < 1.0:
  FSE is slower than the baseline
```

## Noise threshold

The standard benchmark movement threshold is:

```text
+/- 0.030000
```

Movement within this range is treated as stable/noise unless the commit intentionally changes a structural metric or a repeated-trial pattern shows consistent movement.

Interpretation:

```text
delta >= +0.030000:
  improved

delta <= -0.030000:
  regressed

-0.030000 < delta < +0.030000:
  stable/noise
```

Use repeated-trial artifacts over one noisy single-run result.

## Required evidence for performance claims

A performance claim should use repeated-trial evidence.

Single-run benchmark output is useful for smoke testing, but it should not be used as the primary acceptance basis for a performance commit.

Preferred evidence:

```text
low-gap trial summary
low-gap workload trial summary
target workload trial summary
target workload comparison
count-only workload summary
count-only workload comparison
existence timing summary
materialization mode summary
```

A performance commit should explain:

```text
what changed
which benchmark mode was targeted
which dataset was used
which baseline was used
which selectivity bucket was used
whether validation passed
whether structural counters changed
whether repeated-trial timing improved
whether any regression is acceptable and why
```

## Reporting and tooling commits

Reporting-only, diagnostic-only, and script-only commits should be judged primarily by artifact correctness.

Timing movement alone is not a revert trigger for reporting-only commits.

A reporting/tooling commit should be kept when:

```text
validation passes
expected artifacts are generated
new artifact fields are populated
existing artifact fields remain usable
scripts still run successfully
comparison artifacts remain parseable
review manifests correctly report found/skipped artifacts
```

A reporting/tooling commit should be fixed or reverted when:

```text
required artifacts are missing
CSV files are malformed
review scripts fail
manifest states are wrong
existing comparison workflows break
validation no longer runs
```

## Query behavior commits

Query behavior commits must preserve correctness unless the commit explicitly changes semantics.

Required checks:

```text
owned-result matched records remain exact
count-only matched records remain exact
count-only matched records equal owned-result matched records when comparing the same query
structural stats remain consistent when expected
validation passes
```

A query behavior commit must not be accepted based only on improved timing if correctness or validation regresses.

## Owned-result benchmark rules

Owned-result benchmark mode measures full query execution including materialized `Vec<Vector>` results.

Use owned-result benchmark artifacts when the caller needs returned records.

Owned-result benchmark evidence should come from:

```text
summary-<label>.csv
workloads-<label>.csv
low-selectivity-gap-<label>.csv
low-gap-trial-summary-<label>.csv
low-gap-workload-trial-summary-<label>.csv
target-workload-trial-summary-<label>.csv
target-workload-trial-comparison-<label>.csv
```

Owned-result timing should not be mixed with count-only timing when making performance claims.

## Count-only benchmark rules

Count-only benchmark mode measures exact cardinality without materializing owned `Vector` results.

Use count-only benchmark artifacts when the caller only needs cardinality.

The public count-only APIs are:

```text
count_query_matches
count_query_matches_with_stats
```

Count-only benchmark evidence should come from:

```text
count-only-workload-summary-<label>.csv
count-only-workload-summary-<label>.txt
count-only-workload-summary-comparison-<label>.csv
count-only-workload-summary-comparison-<label>.txt
```

The most important correctness field is:

```text
all_stats_match_owned
```

Count-only timing should be used as evidence only when:

```text
all_stats_match_owned: true
```

If `all_stats_match_owned` is false, do not use count-only timing for performance conclusions until the mismatch is explained.

## Count-only comparison acceptance rules

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
count-only conclusions remain separate from owned-result and existence conclusions
```

Interpretation rules:

```text
weighted count-only speedup classification:
  higher is better

mean count-only elapsed classification:
  lower is better

owned-result low-gap classification:
  separate benchmark mode
```

Do not use count-only comparison artifacts for:

```text
claiming owned-result query speedups
claiming materialized row-return performance improved
accepting traversal changes without owned-result benchmark evidence
```

## Full-selectivity count-only caution

Full-selectivity count-only speedups can be extremely large because count-only root coverage can return total cardinality without materializing all owned `Vector` results.

This is expected.

Do not interpret full-selectivity count-only speedup as a general traversal improvement.

Low-selectivity count-only rows are usually more useful for selective-query movement because they preserve selective query behavior while removing owned result construction.

## Boundary workload optimization rules

The small target boundary workload is:

```text
cluster_boundary_range
```

The large target boundary workload is:

```text
large_cross_cluster_boundary
```

Boundary-specific optimization should not be accepted unless repeated-trial evidence supports it.

Do not target these without new repeated-trial evidence:

```text
sibling overlap tuning
boundary-specific traversal specialization
partial-leaf materialization fusion
covered-leaf fast paths for cluster_boundary_range
default leaf policy switch from 8/8 to 4/8
```

Reasons:

```text
sibling overlap is repeatedly zero
cluster_boundary_range has no covered leaves
candidate records are already very low
small boundary workload is near the fixed-cost floor under owned-result execution
large target shows owned Vector materialization dominates retained execution
```

## Boundary work acceptance rule

For future boundary work, acceptance requires:

```text
validation passes
matched records remain exact
candidate count changes are intentionally explained
kd_tree target workload timing improves by at least +0.030000
aggregate low-selectivity timing does not regress beyond -0.030000
FSE visited nodes do not increase unless repeated-trial timing justifies it
FSE reconstructed records do not increase unless intentionally explained
```

## Leaf policy acceptance rules

The current default leaf policy remains:

```text
target leaf size: 8
max leaf size: 8
```

Do not switch the small benchmark default from `8/8` to `4/8` without new repeated-trial evidence.

Current conclusion:

```text
keep 8/8
do not switch the small benchmark default to 4/8
4/8 reduces candidate work but increases traversal pressure and regresses timing
```

A leaf policy change must show:

```text
validation passes
leaf cardinality remains valid
aggregate repeated-trial timing improves
target workload timing improves or remains stable
traversal pressure does not increase enough to erase candidate savings
large and small datasets are evaluated separately
```

## Dataset-specific acceptance rules

Small dataset review baseline:

```text
dataset: small
max depth: 8
target workload: cluster_boundary_range
expected validation: pass
```

Large dataset review baseline:

```text
dataset: large
max depth: 16
target workload: large_cross_cluster_boundary
expected validation: pass
```

Large dataset max depth 8 is not an acceptable default baseline for max leaf size 8 because it can stop recursive construction before leaf cardinality is valid.

Invalid large-depth-8 shape:

```text
records: 10000
max depth: 8
max leaf size: 8
leaf records max: 63
leaf records avg: 39.06
validation: fail
leaf cardinality valid: false
```

Valid large-depth-16 shape:

```text
records: 10000
max depth: 16
max leaf size: 8
leaf records max: 8
leaf records avg: 7.81
validation: pass
leaf cardinality valid: true
```

Do not compare large-depth-8 artifacts against large-depth-16 artifacts as equivalent performance baselines.

## Revert rules

Revert or fix a commit when:

```text
validation fails unexpectedly
owned-result correctness changes unintentionally
count-only correctness changes unintentionally
CSV artifacts become malformed
review scripts fail
required artifacts are missing
comparison artifacts silently skip required inputs
structural counters change without explanation
performance regresses beyond threshold in repeated-trial evidence for a performance-targeting commit
```

Do not revert a reporting-only commit solely because timing moved, as long as:

```text
query behavior did not change
validation passes
artifact output is correct
structural counters remain stable
```

## Minimum review checklist

Before accepting a benchmark-related commit, check:

```text
cargo fmt completed
cargo test passed
cargo check --release passed
benchmark validation passed
index validation passed
leaf cardinality validation passed
expected artifacts were generated
review manifest is accurate
owned-result, count-only, and existence conclusions are interpreted separately
single-run timing is not overinterpreted
repeated-trial evidence is used for performance claims
```

## Existence-related benchmark evidence

Existence execution is a separate output contract.

It answers:

```text
is |E(Q, F)| greater than zero
```

Existence timing should not be treated as count-only timing.

The existence API is public. Its benchmark evidence currently stays in existence timing summary artifacts and debug review output. Durable previous/current comparison tracking is deferred until repeated-trial evidence shows that the boolean output contract needs review tracking separate from count-only.

Acceptance requirements for existence evidence:

```text
existence matches owned presence: true
existence matches count-only presence: true
all existence results agree: true
```

For workload-level existence debug summaries, every row should report:

```text
agreement = pass
```
