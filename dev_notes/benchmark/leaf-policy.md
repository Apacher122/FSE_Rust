# Leaf Policy Notes

This file documents leaf policy decisions for the FSE Rust benchmark workflow.

Leaf policy affects index shape, traversal pressure, candidate reconstruction work, retained leaf count, and timing. It should be treated as a benchmark-relevant configuration, not a cosmetic tuning parameter.

## Current leaf policy

The current default policy is:

    target leaf size: 8
    max leaf size: 8

Short label:

    8/8

Current conclusion:

    keep 8/8
    do not switch the default small benchmark policy to 4/8
    do not switch the default large benchmark policy without separate repeated-trial evidence

## Leaf policy meaning

The target leaf size controls the preferred stopping point for recursive partitioning.

The max leaf size controls the hard validation limit for leaf cardinality, unless the build depth limit prevents further splitting.

The current valid benchmark baselines are:

    small dataset:
      max depth: 8
      target leaf size: 8
      max leaf size: 8
      validation: pass

    large dataset:
      max depth: 16
      target leaf size: 8
      max leaf size: 8
      validation: pass

The large dataset needs max depth 16 for the 8/8 policy to satisfy leaf cardinality validation.

## Why 4/8 was investigated

The 4/8 policy was investigated because the small boundary workload had retained candidate work near the decision boundary.

Hypothesis:

    smaller target leaves may reduce candidate records
    reduced candidate records may improve low-selectivity workload timing
    max leaf size 8 would still preserve the hard validation limit

The tested alternate policy was:

    target leaf size: 4
    max leaf size: 8

Short label:

    4/8

## 4/8 result summary

The 4/8 policy reduced some candidate work, but it increased traversal pressure.

Observed direction:

    candidate records decreased
    retained leaves increased
    index nodes increased
    traversal pressure increased
    timing regressed

Conclusion:

    4/8 is not the better default for the current small benchmark

## Why 8/8 remains the default

The 8/8 policy remains the default because repeated trials showed it was the better tradeoff for the current benchmark workflow.

Reasoning:

    4/8 reduced candidate work
    but 4/8 increased traversal pressure
    the traversal increase erased or outweighed candidate savings
    repeated timing did not justify switching defaults
    the small boundary workload is already close to the practical fixed-cost floor

Current decision:

    keep 8/8 as the default policy

## Relationship to boundary diagnostics

The boundary diagnostics showed:

    sibling overlap is repeatedly zero
    cluster_boundary_range has no covered leaves
    candidate records are already very low
    predicate checking is cheap
    owned Vector materialization is a visible cost
    tiny-query fixed overhead dominates the small target workload

Because of this, reducing leaf size is not automatically beneficial.

For the small boundary target:

    retained leaves: 2
    candidate records: 10
    matched records: 5
    covered leaves: 0
    partial leaves: 2

This means:

    candidate count is already low
    covered-leaf fast paths do not help this workload
    smaller leaves risk increasing traversal without enough candidate savings

## Relationship to large dataset baseline

The valid large dataset baseline is:

    dataset: large
    max depth: 16
    target leaf size: 8
    max leaf size: 8
    validation: pass

The large target workload shape under the valid depth-16 baseline is:

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

Do not generalize the small 4/8 result to the large dataset without separate repeated-trial evidence.

If a future leaf policy experiment targets the large dataset, it should use the valid large baseline:

    max depth: 16

not the old invalid depth-8 large shape.

## Invalid large depth caution

The large dataset with max depth 8 was invalid for max leaf size 8.

Invalid shape:

    records: 10000
    max depth: 8
    max leaf size: 8
    leaf records max: 63
    leaf records avg: 39.06
    validation: fail
    leaf cardinality valid: false

Do not use invalid large-depth-8 artifacts as leaf policy evidence.

Large leaf policy experiments must preserve:

    validation: pass
    leaf cardinality valid: true

## Leaf policy acceptance rule

A leaf policy change must satisfy:

    validation passes
    leaf cardinality remains valid
    owned-result matched records remain exact
    count-only matched records remain exact when relevant
    repeated-trial timing improves
    aggregate low-selectivity timing does not regress beyond the noise threshold
    target workload timing improves or remains stable
    traversal pressure does not increase enough to erase candidate savings
    small and large datasets are evaluated separately

The standard movement threshold is:

    +/- 0.030000

Interpretation:

    delta >= +0.030000:
      improved

    delta <= -0.030000:
      regressed

    -0.030000 < delta < +0.030000:
      stable/noise

## Required evidence for a future leaf policy change

A future leaf policy change should provide repeated-trial evidence for:

    small dataset, if changing the small default
    large dataset, if changing the large default
    low-selectivity aggregate movement
    weakest workload movement
    target workload movement
    candidate ratio movement
    visited node movement
    retained leaf movement
    reconstructed record movement
    validation state

Useful artifacts:

    low-gap-trial-summary-<label>.csv
    low-gap-workload-trial-summary-<label>.csv
    target-workload-trial-summary-<label>.csv
    target-workload-trial-comparison-<label>.csv
    workloads-<label>.csv
    count-only-workload-summary-<label>.csv
    count-only-workload-summary-<label>.txt

## Do not switch leaf policy based on one run

Do not switch the default leaf policy based on:

    one smoke run
    one target workload row
    one single-run timing ratio
    candidate reduction alone
    count-only speedup alone
    invalid large-depth artifacts
    debug-only retained diagnostics alone

Leaf policy changes alter index shape. They need repeated evidence.

## Candidate reduction is not enough

Candidate reduction is useful only if it improves end-to-end timing.

A leaf policy that reduces candidates can still be worse if it increases:

    visited nodes
    retained leaves
    traversal work
    bounds checks
    report overhead
    tiny-query fixed overhead

The 4/8 experiment demonstrated this risk.

## Count-only interpretation

Count-only query mode should be reviewed separately from owned-result query mode.

A leaf policy could theoretically improve count-only timing while not improving owned-result timing, or the reverse.

For leaf policy work, keep these conclusions separate:

    owned-result benchmark:
      measures materialized row-return performance

    count-only benchmark:
      measures exact-cardinality performance

Do not claim an owned-result leaf policy improvement from count-only timing alone.

## Future leaf policy experiments

Future experiments may still be worthwhile, but they should be explicit.

Possible future experiments:

    adaptive leaf policy by dataset size
    adaptive leaf policy by estimated query selectivity
    separate target and max leaf sizes for large datasets
    build-depth-aware leaf policy validation
    result-mode-aware leaf policy review
    larger dataset policy sweep

Each experiment should use a clear label and preserve comparability.

Example labels:

    leaf-policy-small-8-8-baseline
    leaf-policy-small-4-8-review
    leaf-policy-large-8-8-baseline
    leaf-policy-large-4-8-review
    leaf-policy-large-16-depth-review

## Suggested review commands for future policy experiments

Small policy review:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "small" `
      -Trials 5 `
      -Iterations 10000

Large policy review:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000

Large review with organized copy:

    .\scripts\run-low-gap-review.ps1 `
      -Label "<label>" `
      -PreviousLabel "<previous-label>" `
      -Dataset "large" `
      -Trials 5 `
      -Iterations 10000 `
      -CopyArtifactsToRunFolder

## Current policy conclusion

The current benchmark posture is:

    keep target leaf size 8
    keep max leaf size 8
    keep small max depth 8
    keep large max depth 16
    do not switch to 4/8 without new repeated-trial evidence

Current short decision:

    keep 8/8

Reason:

    8/8 is the best-supported current default
    4/8 increased traversal pressure
    4/8 did not produce enough repeated-trial timing benefit
    boundary diagnostics point to fixed overhead and result ownership, not leaf size, as the next major issue
    