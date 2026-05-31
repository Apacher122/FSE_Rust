# Reference Visitor Query Workflow

This file records the reference visitor output contract and the first benchmark evidence for its materialization cost.

The reference visitor API visits exact row references one at a time. It does not build a `Vec<QueryResultReference>` for the full result set.

## Output contract

The reference visitor is an exact output contract over the same query result as owned-result, reusable-owned, reference-result, count-only, and existence execution.

It preserves the staged execution semantics:

```text
Geometry -> Reconstruction -> Logic
```

The visitor changes only the result delivery shape:

```text
reference-result:
  returns Vec<QueryResultReference>

reference-visitor:
  calls a visitor function for each exact QueryResultReference
```

Valid reference visitor evidence:

```text
reference visitor stats match reference-result stats
reference visitor matched records match reference-result matched records
materialization agreement = pass
```

Invalid reference visitor evidence:

```text
reference visitor stats differ from reference-result stats
reference visitor matched records differ from reference-result matched records
materialization agreement != pass
```

A mismatch means the timing artifact is invalid until the cause is explained.

## Current benchmark scope

Reference visitor timing is reported through the materialization mode summary.

The summary compares these output contracts:

```text
fresh owned result
reusable owned result
reference-result
reference-visitor
row-view
count-only
```

Generated files:

```text
materialization-mode-summary-<label>.csv
materialization-mode-summary-<label>.txt
materialization-mode-summary-comparison-<label>.csv
materialization-mode-summary-comparison-<label>.txt
```

The summary CSV includes:

```text
reference_elapsed
visitor_elapsed
count_only_elapsed
reference_speedup
visitor_speedup
agreement
```

Row-view fields are tracked in the same materialization summary after the row-view API landed. The reference visitor conclusions in this file remain about reference delivery, not borrowed row-view delivery.

The debug output includes a target workload section showing reference visitor elapsed time, estimated reference-result overhead above reference visitor, and whether visitor stats match reference-result stats.

## Small dataset evidence: reference visitor materialization

The first repeated small-dataset review used:

```text
dataset: small
trials: 5
iterations: 1000
target workload: cluster_boundary_range
```

The materialization summary reported `agreement = pass` for every workload.

Visitor versus reference-result timing:

```text
cluster_range_000:
  reference-result: 105ns
  reference-visitor: 79ns
  visitor advantage: 26ns
  reference/visitor ratio: 1.33x

cluster_range_001:
  reference-result: 119ns
  reference-visitor: 90ns
  visitor advantage: 29ns
  reference/visitor ratio: 1.32x

cluster_range_002:
  reference-result: 112ns
  reference-visitor: 92ns
  visitor advantage: 20ns
  reference/visitor ratio: 1.22x

empty_far_range:
  reference-result: 22ns
  reference-visitor: 23ns
  visitor result: noise-equivalent, 1ns slower

full_dataset_range:
  reference-result: 72ns
  reference-visitor: 65ns
  visitor advantage: 7ns
  reference/visitor ratio: 1.11x

cluster_boundary_range:
  reference-result: 133ns
  reference-visitor: 108ns
  visitor advantage: 25ns
  reference/visitor ratio: 1.23x
```

Conclusion:

```text
reference visitor is faster than reference-result on 5 of 6 small workloads
empty_far_range is noise-equivalent
reference visitor reduces reference-result allocation/output overhead in this run
reference visitor does not replace count-only when the caller only needs cardinality
```

## Target workload interpretation

For `cluster_boundary_range`, the debug output reported:

```text
fresh owned average elapsed: 243ns
reusable owned average elapsed: 134ns
reference-result average elapsed: 146ns
reference-visitor average elapsed: 105ns
count-only average elapsed: 104ns
estimated reference-result above reference-visitor: 41ns
reference visitor stats match reference: true
all matched records agree: true
```

Interpretation:

```text
reference visitor is close to count-only on this selective target workload
reference-result Vec construction remains visible at this size
owned-result materialization remains more expensive than reference delivery
```

## Large dataset evidence: reference visitor materialization

The large-dataset review used:

```text
dataset: large
trials: 5
iterations: 1000
max depth: 16
target workload: large_cross_cluster_boundary
```

The materialization summary reported `agreement = pass` for every workload.

Visitor versus reference-result timing:

```text
large_cluster_range_000:
  reference-result: 267ns
  reference-visitor: 254ns
  visitor advantage: 13ns
  reference/visitor ratio: 1.05x

large_cluster_range_001:
  reference-result: 252ns
  reference-visitor: 233ns
  visitor advantage: 19ns
  reference/visitor ratio: 1.08x

large_cluster_range_002:
  reference-result: 273ns
  reference-visitor: 243ns
  visitor advantage: 30ns
  reference/visitor ratio: 1.12x

large_cluster_range_003:
  reference-result: 233ns
  reference-visitor: 211ns
  visitor advantage: 22ns
  reference/visitor ratio: 1.10x

large_cluster_range_004:
  reference-result: 256ns
  reference-visitor: 248ns
  visitor advantage: 8ns
  reference/visitor ratio: 1.03x

large_cluster_range_005:
  reference-result: 224ns
  reference-visitor: 207ns
  visitor advantage: 17ns
  reference/visitor ratio: 1.08x

large_cluster_range_006:
  reference-result: 237ns
  reference-visitor: 230ns
  visitor advantage: 7ns
  reference/visitor ratio: 1.03x

large_cluster_range_007:
  reference-result: 225ns
  reference-visitor: 203ns
  visitor advantage: 22ns
  reference/visitor ratio: 1.11x

large_cluster_range_008:
  reference-result: 258ns
  reference-visitor: 278ns
  visitor result: 20ns slower

large_cluster_range_009:
  reference-result: 238ns
  reference-visitor: 217ns
  visitor advantage: 21ns
  reference/visitor ratio: 1.10x

large_empty_far_range:
  reference-result: 22ns
  reference-visitor: 22ns
  visitor result: equal

large_full_dataset_range:
  matched records: 10000
  reference-result: 11.656us
  reference-visitor: 5.625us
  visitor advantage: 6.031us
  reference/visitor ratio: 2.07x

large_cross_cluster_boundary:
  matched records: 71
  reference-result: 374ns
  reference-visitor: 332ns
  visitor advantage: 42ns
  reference/visitor ratio: 1.13x
```

Conclusion:

```text
reference visitor is faster than reference-result on 11 of 13 large workloads
large_empty_far_range is equal at 22ns
the only slower visitor row is large_cluster_range_008 at 20ns slower
large_full_dataset_range shows the clearest visitor benefit
reference visitor reduces reference-result allocation/output overhead when many references must be delivered
reference visitor still does not replace count-only when the caller only needs cardinality
```

## Large target workload interpretation

For `large_cross_cluster_boundary`, the materialization summary reported:

```text
matched records: 71
reference-result: 374ns
reference-visitor: 332ns
count-only: 283ns
agreement: pass
```

The target debug section uses an independent timing probe and reported:

```text
fresh owned average elapsed: 2.06us
reference-result average elapsed: 363ns
reference-visitor average elapsed: 347ns
count-only average elapsed: 304ns
estimated reference-result above reference-visitor: 16ns
reference visitor stats match reference: true
all matched records agree: true
```

Interpretation:

```text
large_cross_cluster_boundary is still near the noise boundary for reference-result versus visitor timing
both the materialization summary and target debug probe reported lower visitor timing in this run
both probes preserve exact agreement
count-only remains lower because it does not deliver references
large_full_dataset_range is the stronger large-dataset evidence for visitor materialization savings
```

## Limits

These materialization runs are useful output-contract evidence, not broad FSE performance proof.

Do not claim:

```text
reference visitor is universally faster
reference visitor improves full FSE query performance
reference visitor is universally faster on large data
reference visitor replaces count-only for resultless query use cases
```

The next durable benchmark step should be either:

```text
repeated materialization-summary-specific trial workflow
large dataset comparison against a previous visitor run
```

Use visitor timing as output-contract evidence, not as proof that geometric pruning improved.
