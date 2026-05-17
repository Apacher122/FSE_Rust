# Fractal Semantic Encoding Rust Implementation

This repository contains a Rust implementation of Fractal Semantic Encoding, or FSE.
The current goal of the project is to build a correct and measurable FSE query engine that can compete against conventional exact range-query baselines.

## Current implementation focus

The implementation is currently focused on:

- preserving exact query correctness
- validating reconstruction against original points
- keeping query execution staged and measurable
- comparing FSE against exact baselines
- making serial and parallel retained-leaf execution selectable
- keeping benchmark output compact by default
- keeping detailed workload output behind debug/report flags

The active query execution pipeline is:

```text
Stage I: geometric traversal
Stage II: deferred reconstruction
Stage III: exact evaluation
Final: deterministic merge