# Exact Existence Query Workflow

This file documents the exact existence query output contract and its current benchmark evidence scope.

Existence queries evaluate whether the exact FSE result set is non-empty. The implementation uses the same staged execution semantics as owned-result, reference-result, reference-visitor, and count-only queries.

## Formal basis

The FSE paper defines exact execution as:

```text
E(Q, F) = σ_Q(Φ(R_T(Q)))
```

The staged execution order is:

```text
Geometry -> Reconstruction -> Logic
```

Existence is the boolean projection over that exact result set:

```text
query_has_match(Q, F) = |E(Q, F)| > 0
```

Equivalently:

```text
query_has_match(Q, F) <=> there exists x in E(Q, F)
```

The existence path may short-circuit after it establishes the boolean result. Partial retained leaves still require exact predicate evaluation through `σ_Q`. A covered non-empty subtree may produce `true` without row-level predicate checks because coverage proves containment for the subtree.

## Public API

The public exact existence API is:

```text
query_has_match
```

The reporting API is:

```text
query_has_match_with_stats
```

The reporting API returns:

```text
has_match
inspected_records
stats
```

`inspected_records` is the number of candidate rows inspected before the existence result was established. For a positive query, this may be lower than the retained candidate count. For a disjoint query, it may be zero.

## Relationship to other output contracts

Owned-result execution returns:

```text
Vec<Vector>
```

Count-only execution returns:

```text
exact cardinality
```

Reference-result execution returns:

```text
leaf and row references for exact matches
```

Existence execution returns:

```text
bool
```

All non-existence output contracts preserve the same query semantics.

Valid existence evidence:

```text
owned-result has match == count-only has match == existence has match
```

Invalid existence evidence:

```text
owned-result has match != existence has match
count-only has match != existence has match
```

A mismatch between existence and either owned-result presence or count-only presence is a correctness failure. Timing evidence from that artifact is invalid until the mismatch is explained.

## Current benchmark scope

Existence timing is reported in debug output and extracted into summary artifacts by the benchmark script.

The debug report includes:

```text
Target workload exact existence timing
Workload exact existence timing summary
```

The target section includes:

```text
fresh owned average elapsed
count-only average elapsed
existence average elapsed
existence inspected records
existence speedup vs fresh owned
existence speedup vs count-only
existence matches owned presence
existence matches count-only presence
all existence results agree
```

The workload summary includes:

```text
workload
owned has match
count-only has match
existence has match
existence inspected records
fresh owned
count-only
existence
existence speedup vs owned
existence speedup vs count-only
agreement
```

The current evidence supports existence as a public query API and benchmark summary artifact. Review-manifest comparison artifacts are deferred.

## Summary artifacts

The benchmark script extracts workload-level existence timing into:

```text
existence-timing-summary-<label>.csv
existence-timing-summary-<label>.txt
```

The CSV contains:

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

The notes file records row count, agreement failures, total count-only candidates, total inspected records, total skipped records, and the aggregate inspection ratio.

## Current evidence summary

Commit 341 added inspected-record evidence for exact existence execution. Commit 347 added candidate reduction metrics that compare existence inspection against count-only candidate work.

Small dataset:

```text
all workloads: agreement = pass
empty_far_range: false/false/false, inspected records 0
full_dataset_range: true/true/true, inspected records 1
cluster_boundary_range: true/true/true, inspected records 4
cluster_boundary_range: existence speedup vs owned about 2.58x
cluster_boundary_range: existence speedup vs count-only about 1.05x
```

Large dataset:

```text
all workloads: agreement = pass
large_empty_far_range: false/false/false, inspected records 0
large_full_dataset_range: true/true/true, inspected records 1
large_cross_cluster_boundary: true/true/true, inspected records 5
large_cross_cluster_boundary: existence speedup vs owned about 7.34x
large_cross_cluster_boundary: existence speedup vs count-only about 1.02x
```

Interpretation:

```text
existence is semantically valid across the current benchmark workloads
existence avoids owned result materialization
positive full-range workloads inspect one candidate record before returning true
empty disjoint workloads inspect zero candidate records
existence remains close to count-only on the current positive boundary workloads
existence summary artifacts preserve inspected-record and skipped-record evidence
```

## Promotion rule

Existence comparison scripts and review-manifest entries should be added only when the benchmark evidence requires durable review separate from count-only artifacts.

A future existence artifact pipeline is justified when at least one condition applies:

```text
existence materially outperforms count-only on repeated trials
existence becomes part of a user-facing benchmark mode
existence summary artifacts require previous/current regression tracking
existence drives an implementation decision that count-only evidence cannot support
```

Current scope:

```text
public query API
debug benchmark evidence
existence timing summary artifacts
review-manifest comparison artifacts deferred
```

## Acceptance rule

Existence-related commits should preserve:

```text
query_has_match == (count_query_matches > 0)
query_has_match == !execute_query(...).is_empty()
```

Benchmark debug output must show:

```text
existence matches owned presence: true
existence matches count-only presence: true
all existence results agree: true
```

For workload-level summaries, every row must have:

```text
agreement = pass
```

`inspected_records` must be interpreted as existence-path work, not as total retained candidate cardinality. Positive existence queries may inspect fewer retained candidates than count-only queries because the result can be established after the first exact match.
