# Dataset Baselines and Target Workloads

This file documents the benchmark datasets, default review depths, and target workloads used by the FSE Rust benchmark workflow.

Dataset and target workload settings are part of benchmark interpretation. Do not compare benchmark artifacts from incompatible dataset or build-depth configurations as if they were equivalent.

## Dataset review baselines

The benchmark review workflow is dataset-selectable.

Supported datasets:

```text
small
large
```

The small dataset baseline is:

```text
dataset: small
max depth: 8
target workload: cluster_boundary_range
expected validation: pass
```

The large dataset baseline is:

```text
dataset: large
max depth: 16
target workload: large_cross_cluster_boundary
expected validation: pass
```

The review scripts default to these dataset-specific target workloads:

```text
small dataset:
  cluster_boundary_range

large dataset:
  large_cross_cluster_boundary
```

## Small dataset baseline

The small dataset is the original clustered 2D benchmark workload.

Expected small benchmark shape:

```text
dataset: small
records: 60
max depth: 8
target leaf size: 8
max leaf size: 8
validation: pass
leaf cardinality valid: true
```

The small target workload is:

```text
cluster_boundary_range
```

The stable small target workload shape is:

```text
query min: [18.00, 18.00]
query max: [52.00, 52.00]
retained leaves: 2
```

The retained leaves are:

```text
leaf 7:
  coverage: partial
  records: 5
  bounds min: [15.00, 15.00]
  bounds max: [19.00, 19.00]
  volume: 16.00

leaf 11:
  coverage: partial
  records: 5
  bounds min: [50.00, 50.00]
  bounds max: [54.00, 54.00]
  volume: 16.00
```

The stable small target structural counters are:

```text
FSE visited nodes: 13
FSE retained leaves: 2
FSE reconstructed records: 10
FSE matched records: 5
candidate ratio: 0.166667
nodes per record: 1.300000
covered leaves: 0
partial leaves: 2
covered records: 0
partial records: 10
```

Use the small dataset when checking regressions against the original boundary workload.

## Large dataset baseline

The large dataset is the larger clustered 2D benchmark workload.

Expected large benchmark shape:

```text
dataset: large
records: 10000
max depth: 16
target leaf size: 8
max leaf size: 8
validation: pass
leaf cardinality valid: true
```

The valid large-depth-16 shape is:

```text
records: 10000
max depth: 16
max leaf size: 8
leaf records max: 8
leaf records avg: 7.81
validation: pass
leaf cardinality valid: true
```

The large target workload is:

```text
large_cross_cluster_boundary
```

The valid large target workload shape is:

```text
target workload: large_cross_cluster_boundary
FSE visited nodes: 45
FSE retained leaves: 10
FSE reconstructed records: 78
FSE matched records: 71
candidate ratio: 0.007800
covered leaves: 8
partial leaves: 2
covered records: 62
partial records: 16
```

Use the large dataset when checking whether candidate reduction dominates tiny-query fixed overhead.

Use the large dataset when evaluating count-only behavior at a larger scale.

## Invalid large-depth-8 baseline

The large dataset should not use max depth 8 as the default review baseline when max leaf size is 8.

With 10,000 records, max depth 8 can stop recursive construction before the max leaf cardinality constraint is satisfied.

The invalid large-depth-8 shape was:

```text
records: 10000
max depth: 8
max leaf size: 8
leaf records max: 63
leaf records avg: 39.06
validation: fail
leaf cardinality valid: false
```

The previous large-depth-8 target shape was:

```text
target workload: large_cross_cluster_boundary
FSE visited nodes: 27
FSE retained leaves: 3
FSE reconstructed records: 94
FSE matched records: 71
candidate ratio: 0.009400
validation: fail
leaf cardinality valid: false
```

The large-depth-8 target shape is not comparable as a performance baseline because the index was invalid.

Do not compare large-depth-8 artifacts against large-depth-16 artifacts as if they were equivalent performance runs.

## Large depth change interpretation

Changing large review depth from 8 to 16 changed the large index shape.

Before depth fix:

```text
dataset: large
max depth: 8
validation: fail
leaf records max: 63
target retained leaves: 3
target candidate records: 94
```

After depth fix:

```text
dataset: large
max depth: 16
validation: pass
leaf records max: 8
target retained leaves: 10
target candidate records: 78
```

Future large-dataset comparisons should use the valid depth-16 baseline unless a commit intentionally tests build-depth behavior.

## Target workload review options

The review runner chooses the default target workload from the selected dataset:

```text
small dataset: cluster_boundary_range
large dataset: large_cross_cluster_boundary
```

The target workload can be manually overridden when intentionally investigating a different workload.

Manual target summary example:

```powershell
.\scripts\benchmark\summarize\summarize-target-workload-trials.ps1 `
  -Label "<commit-label>" `
  -InputLabel "<commit-label>" `
  -TargetWorkloadName "<target-workload-name>"
```

Manual review override example:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<commit-label>" `
  -Dataset "large" `
  -TargetWorkloadName "<target-workload-name>" `
  -Trials 5 `
  -Iterations 10000
```

Only override the target workload when the commit intentionally changes the review target or investigates a specific workload.

## Manual max-depth override guidance

Manual max-depth override is allowed only when the commit is intentionally testing build-depth behavior or comparing alternate valid build shapes.

Example:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<commit-label>" `
  -Dataset "large" `
  -MaxDepth 16 `
  -Trials 5 `
  -Iterations 10000
```

Do not use max-depth overrides casually. They change index shape and can make performance artifacts non-comparable.

When max depth changes, the review notes should explain:

```text
why max depth changed
whether validation passed
how index shape changed
whether target workload counters changed
whether old artifacts remain comparable
```

## Preferred review commands

Preferred small review command:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<commit-label>" `
  -PreviousLabel "<previous-label>" `
  -Dataset "small" `
  -Trials 5 `
  -Iterations 10000
```

Preferred large review command:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<commit-label>" `
  -PreviousLabel "<previous-label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000
```

Preferred large review command with organized artifact copy:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<commit-label>" `
  -PreviousLabel "<previous-label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000 `
  -CopyArtifactsToRunFolder
```

## Dataset-specific interpretation rules

Small dataset interpretation:

```text
good for original boundary workload regression checks
good for tiny-query fixed-cost investigation
more sensitive to nanosecond-scale noise
should not drive large-scale conclusions by itself
```

Large dataset interpretation:

```text
good for larger candidate-reduction evidence
good for count-only versus owned-result output-contract comparison
requires max depth 16 for valid max leaf size 8 baseline
should be used for broader performance direction
```

Do not generalize from one dataset alone when making project-level performance claims.

## Dataset acceptance rules

Small dataset acceptance requires:

```text
dataset: small
max depth: 8
validation: pass
leaf cardinality valid: true
target workload: cluster_boundary_range
```

Large dataset acceptance requires:

```text
dataset: large
max depth: 16
validation: pass
leaf cardinality valid: true
target workload: large_cross_cluster_boundary
```

If dataset, max depth, or target workload changes, document the reason before comparing performance movement.

## Current dataset posture

The current dataset posture is:

```text
small:
  stable original boundary workload
  useful for regression checks
  dominated by tiny-query fixed overhead under owned-result execution

large:
  valid depth-16 review baseline
  useful for broader benchmark direction
  confirms owned Vector materialization dominates retained execution for target workload
  useful for count-only query mode evaluation
```
