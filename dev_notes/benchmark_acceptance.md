# Benchmark Acceptance Notes

The benchmark notes have been split by topic so the review workflow is easier to maintain.

Use this file as the entry point.

## Topic files

| Topic | File |
|---|---|
| Acceptance thresholds and performance decision rules | `dev_notes/benchmark/acceptance-rules.md` |
| Dataset baselines, target workloads, and review depths | `dev_notes/benchmark/datasets-and-targets.md` |
| Boundary diagnostics, fixed-cost conclusions, and result ownership conclusions | `dev_notes/benchmark/diagnostic-conclusions.md` |
| Count-only query timing, summary artifacts, and comparison artifacts | `dev_notes/benchmark/count-only-workflow.md` |
| Exact existence query timing and artifact-scope decision | `dev_notes/benchmark/existence-workflow.md` |
| Low-gap review runner, target workload review, and comparison commands | `dev_notes/benchmark/review-workflow.md` |
| Artifact organization, organized previous-label resolution, organized copy, and cleanup | `dev_notes/benchmark/artifact-workflow.md` |
| Leaf policy review and current policy conclusion | `dev_notes/benchmark/leaf-policy.md` |

## Current phase

The project is currently in the benchmarking, diagnostic tooling, and measurement-discipline phase.

The core FSE implementation is already benchmarked against:

```text
flat scan
KD-tree
R-tree
```

The current benchmark workflow separates these output contracts:

```text
owned-result query execution:
  returns materialized Vec<Vector> results

count-only query execution:
  returns exact cardinality without owned Vector materialization

existence query execution:
  returns whether the exact result set is non-empty
```

## Preferred review command

Small dataset:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<label>" `
  -PreviousLabel "<previous-label>" `
  -Dataset "small" `
  -Trials 5 `
  -Iterations 10000
```

Large dataset:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<label>" `
  -PreviousLabel "<previous-label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000
```

With organized artifact copy:

```powershell
.\scripts\benchmark\review\run-low-gap-review.ps1 `
  -Label "<label>" `
  -PreviousLabel "<previous-label>" `
  -Dataset "large" `
  -Trials 5 `
  -Iterations 10000 `
  -CopyArtifactsToRunFolder
```

## Important rules

Use repeated-trial evidence, not one noisy benchmark run.

Reporting/tooling-only commits should be judged by artifact correctness, not timing movement alone.

Performance commits should be accepted only when correctness is preserved and repeated-trial evidence supports the change.

Count-only benchmark conclusions must stay separate from owned-result benchmark conclusions.

Existence benchmark conclusions must stay separate from count-only conclusions unless a commit explicitly explains why the boolean output contract needs durable artifact review.
