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
