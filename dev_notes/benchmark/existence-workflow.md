# Exact Existence Query Workflow

This file documents the exact existence query output contract and its current benchmark evidence scope.

Existence queries reuse the exact FSE execution semantics used by owned-result and count-only queries. The returned value reports whether the exact result set `E(Q, F)` is non-empty.

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

Existence preserves the same exact semantics as owned-result, reference-result, and count-only execution. The implementation may short-circuit after finding the first exact match. Partial retained leaves still require exact predicate evaluation through `σ_Q`.

A covered non-empty subtree may produce `true` without row-level predicate checks because the coverage classification proves containment. A partial retained subtree cannot produce `true` from geometric retention alone.

## Public API

The public exact existence API is:

```text
query_has_match
```

The API returns whether the exact query result set contains at least one record.

Record identity, exact cardinality, and storage references are provided by the owned-result, count-only, and reference-result APIs.

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

All four output contracts preserve the same query semantics.

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

Existence timing is currently debug-only.

The debug report includes:

```text
Target workload exact existence timing
Workload exact existence timing summary
```

The workload summary includes:

```text
workload
owned has match
count-only has match
existence has match
fresh owned
count-only
existence
existence speedup vs owned
existence speedup vs count-only
agreement
```

The current evidence supports existence as a public query API and debug benchmark signal. Durable CSV and review-manifest artifacts are deferred.

## Current evidence summary

Commit 338 added workload-level debug evidence for small and large datasets.

Small dataset:

```text
all workloads: agreement = pass
empty_far_range: false/false/false
full_dataset_range: existence speedup vs owned about 67.86x
cluster_boundary_range: existence speedup vs owned about 2.61x
cluster_boundary_range: existence speedup vs count-only about 1.12x
```

Large dataset:

```text
all workloads: agreement = pass
large_empty_far_range: false/false/false
large_full_dataset_range: existence speedup vs owned about 11854.48x
large_cross_cluster_boundary: existence speedup vs owned about 5.42x
large_cross_cluster_boundary: existence speedup vs count-only about 0.79x
```

Interpretation:

```text
existence is semantically valid across the current benchmark workloads
existence avoids owned result materialization
existence has not shown enough distinct value over count-only to require a separate artifact pipeline
```

## Promotion rule

Existence summary CSVs, comparison scripts, and review-manifest entries should be added only when the benchmark evidence requires durable review separate from count-only artifacts.

A future existence artifact pipeline is justified when at least one of these conditions applies:

```text
existence materially outperforms count-only on repeated trials
existence becomes part of a user-facing benchmark mode
existence requires regression tracking separate from count-only
existence drives an implementation decision that count-only evidence cannot support
```

Current scope:

```text
public query API
debug benchmark evidence
durable artifact review deferred
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
