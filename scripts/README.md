# Benchmark scripts

The scripts folder keeps root-level compatibility wrappers for existing commands. New benchmark work should use the canonical folders under `scripts/benchmark`.

```text
scripts/
  benchmark/
    artifacts/   artifact organization and cleanup workflows
    check/       documentation and workflow preflight checks
    compare/     previous/current artifact comparison scripts
    review/      full benchmark review orchestration and validation
    run/         benchmark runners
    summarize/   artifact summary extractors
  legacy/        retained historical runners
  lib/           shared script libraries
```

Root scripts such as `scripts/run-low-gap-review.ps1` forward to the canonical script paths. This keeps old commands stable while making the benchmark workflow easier to navigate.

## Canonical benchmark entry points

Use the canonical benchmark paths for new commands and documentation:

```text
scripts/benchmark/check/check-benchmark-docs.ps1
scripts/benchmark/check/check-benchmark-workflow.ps1
scripts/benchmark/review/run-low-gap-review.ps1
scripts/benchmark/run/run-small-benchmark.ps1
scripts/benchmark/run/run-low-gap-trials.ps1
scripts/benchmark/run/run-materialization-mode-trials.ps1
scripts/benchmark/summarize/summarize-target-workload-trials.ps1
scripts/benchmark/summarize/summarize-typed-archive-append-rebuild-summary.ps1
scripts/benchmark/summarize/summarize-typed-archive-append-delta-query-summary.ps1
scripts/benchmark/summarize/summarize-typed-archive-append-delta-maintenance-summary.ps1
scripts/benchmark/summarize/summarize-typed-archive-compaction-summary.ps1
scripts/benchmark/summarize/summarize-typed-archive-load-summary.ps1
scripts/benchmark/summarize/summarize-typed-archive-maintenance-summary.ps1
scripts/benchmark/compare/compare-materialization-mode-summary.ps1
scripts/benchmark/artifacts/organize-benchmark-artifacts.ps1
scripts/benchmark/artifacts/cleanup-flat-benchmark-artifacts.ps1
```

Root-level scripts remain compatibility wrappers. They should forward to canonical scripts and should not receive new benchmark workflow logic.
