# Benchmark Diagnostic Conclusions

This file records benchmark diagnostic conclusions from the FSE Rust benchmark workflow.

These conclusions are based on debug-only timing probes, retained-leaf diagnostics, target workload review artifacts, and count-only timing comparisons.

Diagnostic conclusions should guide future work, but they should not be confused with public benchmark modes unless explicitly stated.

## Diagnostic scope

The benchmark diagnostics currently distinguish these layers:

```text
traversal:
  geometric pruning and retained-leaf selection

retained execution:
  candidate reconstruction
  exact predicate evaluation
  owned result collection

resultless retained execution:
  retained-leaf exact counting without owned Vector result materialization

count-only query execution:
  public exact-cardinality query API

existence query execution:
  public exact non-empty-result query API

owned-result query execution:
  public materialized Vec<Vector> query API
```

The main conclusion so far is:

```text
FSE pruning is doing its job on the target workloads
remaining target workload cost is primarily result ownership under the current owned Vector API
```

## Sibling overlap conclusion

Sibling overlap was repeatedly checked during the boundary workload diagnostic phase.

The debug output repeatedly showed:

```text
overlapping sibling pairs: 0
total overlap extent: 0.00
has overlap: false
```

Conclusion:

```text
do not target sibling-overlap tuning as the next optimization
```

Reason:

```text
sibling overlap is not the source of the current boundary workload pressure
```

Any future sibling-overlap work should require new evidence showing overlap has become a real cost driver.

## Small boundary workload retained leaf shape

The small target workload is:

```text
cluster_boundary_range
```

Stable query shape:

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

Stable structural counters:

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

Conclusion:

```text
cluster_boundary_range is geometrically tight
candidate count is already near the practical floor
both retained leaves are partial
covered-leaf fast paths do not help this workload
```

## Boundary fixed-cost conclusion after stage timing diagnostics

Stage timing diagnostics were added to prevent further query hot-path changes from being based on guesses.

The small target workload showed that traversal is not the only meaningful cost:

```text
average traversal elapsed: about 92ns to 94ns
average full FSE elapsed: about 243ns to 263ns
estimated traversal share: about 35% to 39%
estimated non-traversal share: about 61% to 65%
```

Retained execution diagnostics showed that most of the non-traversal cost is retained-leaf execution:

```text
average retained execution elapsed: about 162ns to 171ns
estimated retained execution share: about 65% to 70%
```

The retained execution phase probe should not be interpreted as an additive decomposition because the diagnostic reconstruction path allocates owned intermediate rows.

In particular, the diagnostic reconstruction estimate was larger than the retained execution estimate:

```text
average retained reconstruction elapsed: about 271ns to 272ns
average retained execution elapsed: about 162ns to 167ns
```

Conclusion:

```text
the diagnostic reconstruction probe is allocation-contaminated
do not use that reconstruction number as direct evidence for a reconstruction hot-path optimization
```

## Allocation probe conclusion

The allocation probe showed:

```text
average empty result allocation elapsed: about 20ns
average matched result allocation elapsed: about 42ns
average candidate result allocation elapsed: about 41ns
average vector clone collection elapsed: about 152ns
average retained execution elapsed: about 167ns
```

Useful conclusion:

```text
plain result Vec allocation is small
predicate checks over 10 candidate records are cheap
owned Vector result construction is visible at this query size
the remaining small target workload cost is mostly tiny-query fixed overhead and result ownership cost
```

This refined the earlier boundary diagnosis.

The target workload is geometrically tight, but the remaining cost is not obviously solved by:

```text
tighter pruning
sibling-overlap tuning
another boundary-specific traversal hack
```

## Do-not-target list after boundary diagnostics

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
candidate records are already only 10
matched records are only 5
4/8 reduced candidate work but increased traversal pressure and regressed timing
previous retained-leaf hot-path simplifications regressed timing
```

Current fixed-cost conclusion:

```text
cluster_boundary_range is near the practical floor for the current small benchmark and owned-result API shape
```

## Result ownership timing conclusion

Resultless target workload diagnostics split retained execution into two cases:

```text
owned-result retained execution:
  reconstruct matching rows
  evaluate predicates
  allocate and return owned Vector results

resultless retained execution:
  reconstruct/check candidate values only far enough to count matches
  do not allocate returned Vector results
```

The resultless diagnostic was added to answer whether target workload cost is mostly query work or result materialization work.

Small target workload result:

```text
dataset: small
target workload: cluster_boundary_range
candidate records: 10
matched records: 5
covered leaves: 0
partial leaves: 2
covered records: 0
partial records: 10
average resultless retained elapsed: about 30ns
average retained execution elapsed: about 161ns
estimated result ownership overhead: about 131ns
estimated resultless retained share: about 19%
```

Large target workload result:

```text
dataset: large
target workload: large_cross_cluster_boundary
candidate records: 78
matched records: 71
covered leaves: 8
partial leaves: 2
covered records: 62
partial records: 16
average resultless retained elapsed: about 38ns
average retained execution elapsed: about 1.65us
estimated result ownership overhead: about 1.61us
estimated resultless retained share: about 2%
```

Useful conclusion:

```text
target workload traversal is not the dominant cost
predicate checking is not the dominant cost
plain result Vec allocation is small
owned Vector materialization dominates retained execution when many rows match
```

Future performance work should not be framed as another boundary traversal fix unless new evidence changes this diagnosis.

## Public count-only timing conclusion

The count-only query API provides an exact query cardinality path without returning owned `Vector` results.

Public count-only APIs:

```text
count_query_matches
count_query_matches_with_stats
```

The count-only target comparison measures the public count-only API against the owned-result API.

This differs from the earlier resultless retained diagnostic:

```text
resultless retained diagnostic:
  measures retained-leaf match counting after traversal is already available
  debug-only
  not a public query API

count-only query timing:
  includes root classification
  includes traversal
  includes retained-leaf exact evaluation
  includes stats/report construction
  public query API

owned-result query timing:
  includes root classification
  includes traversal
  includes retained-leaf exact evaluation
  includes owned Vector result materialization
```

Small target workload baseline:

```text
dataset: small
target workload: cluster_boundary_range
owned average elapsed: about 249ns
count-only average elapsed: about 103ns
estimated owned result overhead: about 146ns
count-only speedup: about 2.42x
owned matched records: 5
count-only matched records: 5
matched records agree: true
owned candidate records: 10
count-only candidate records: 10
owned retained leaves: 2
count-only retained leaves: 2
owned visited nodes: 13
count-only visited nodes: 13
```

Large target workload baseline:

```text
dataset: large
target workload: large_cross_cluster_boundary
owned average elapsed: about 2.311us
count-only average elapsed: about 389ns
estimated owned result overhead: about 1.922us
count-only speedup: about 5.94x
owned matched records: 71
count-only matched records: 71
matched records agree: true
owned candidate records: 78
count-only candidate records: 78
owned retained leaves: 10
count-only retained leaves: 10
owned visited nodes: 45
count-only visited nodes: 45
```

Conclusion:

```text
count-only execution confirms that pruning and exact match counting are not the dominant target cost
owned Vector materialization is the dominant target cost when many rows match
the count-only API is a valid separate query mode, not a replacement for owned-result execution
```

## Large target diagnostic conclusion

The large target workload is:

```text
large_cross_cluster_boundary
```

Valid large-depth-16 target shape:

```text
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

Large stage timing showed:

```text
average traversal elapsed: about 289ns to 297ns
average full FSE elapsed: about 1.98us to 1.99us
estimated traversal share: about 14% to 15%
```

Large retained timing showed:

```text
average retained execution elapsed: about 1.65us to 1.67us
estimated retained execution share: about 82% to 85%
```

Large allocation timing showed:

```text
average vector clone collection elapsed: about 1.67us
average retained execution elapsed: about 1.65us to 1.67us
```

Large resultless timing showed:

```text
average resultless retained elapsed: about 37ns to 38ns
average retained execution elapsed: about 1.65us
estimated result ownership overhead: about 1.61us
```

Conclusion:

```text
large target strongly confirms that owned Vector result materialization dominates retained execution
candidate reduction is already effective
result ownership is the next meaningful design axis
```

## Interpretation of count-only versus owned-result timing

Do not compare count-only timing against owned-result timing as if they answer the same application question.

They represent different output contracts:

```text
owned-result timing:
  use when the caller needs materialized rows

count-only timing:
  use when the caller only needs cardinality
```

Valid interpretation:

```text
count-only timing shows exact-cardinality performance
owned-result timing shows materialized row-return performance
difference between them estimates result materialization cost
```

Invalid interpretation:

```text
count-only speedup means owned-result query got faster
count-only full-selectivity speedup means traversal improved
resultless retained timing is public API timing
```

## Exact existence timing conclusion

The exact existence query API answers whether the paper's exact execution result is non-empty:

```text
query_has_match(Q, F) = |E(Q, F)| > 0
```

This is a boolean output contract over the same `Geometry -> Reconstruction -> Logic` execution semantics.

Commit 338 added workload-level debug timing for existence execution. Commit 341 added inspected-record reporting for the existence path.

Small dataset result:

```text
all workloads: agreement = pass
empty_far_range: owned/count/existence all false, inspected records 0
full_dataset_range: owned/count/existence all true, inspected records 1
cluster_boundary_range: owned/count/existence all true, inspected records 4
cluster_boundary_range: existence speedup vs owned about 2.58x
cluster_boundary_range: existence speedup vs count-only about 1.05x
```

Large dataset result:

```text
all workloads: agreement = pass
large_empty_far_range: owned/count/existence all false, inspected records 0
large_full_dataset_range: owned/count/existence all true, inspected records 1
large_cross_cluster_boundary: owned/count/existence all true, inspected records 5
large_cross_cluster_boundary: existence speedup vs owned about 7.34x
large_cross_cluster_boundary: existence speedup vs count-only about 1.02x
```

Conclusion:

```text
existence is semantically valid across the current benchmark workloads
existence avoids owned result materialization
existence can terminate after inspecting fewer records than count-only execution
existence remains close to count-only on the current positive boundary workloads
existence is a valid public output contract
existence timing evidence remains summary/debug evidence rather than a durable comparison track
existence comparison promotion requires repeated-trial evidence that count-only artifacts cannot explain
```

## Reference visitor materialization conclusion

The reference visitor API was introduced to measure exact row-reference delivery without constructing a `Vec<QueryResultReference>`.

The first small-dataset materialization review used:

```text
dataset: small
trials: 5
iterations: 1000
```

The materialization summary reported `agreement = pass` for every workload.

Reference visitor timing was lower than reference-result timing on 5 of 6 workloads:

```text
cluster_range_000: 105ns reference-result, 79ns reference-visitor
cluster_range_001: 119ns reference-result, 90ns reference-visitor
cluster_range_002: 112ns reference-result, 92ns reference-visitor
full_dataset_range: 72ns reference-result, 65ns reference-visitor
cluster_boundary_range: 133ns reference-result, 108ns reference-visitor
```

The only slower row was `empty_far_range`:

```text
reference-result: 22ns
reference-visitor: 23ns
```

That 1ns difference is noise-level.

For the target workload `cluster_boundary_range`, debug output reported:

```text
reference-result average elapsed: 146ns
reference-visitor average elapsed: 105ns
count-only average elapsed: 104ns
reference visitor stats match reference: true
all matched records agree: true
```

Conclusion:

```text
reference visitor reduces reference-result output overhead on the current small materialization summary
reference visitor is close to count-only on the small selective target workload
count-only remains the correct resultless contract when only cardinality is needed
large-dataset visitor evidence now shows the same direction on most materialization rows
large_full_dataset_range is the strongest current visitor result because it avoids collecting 10000 references into a Vec
```

## Large reference visitor materialization conclusion

The large reference visitor review used 5 trials with 1000 iterations and reported `agreement = pass` for all materialization rows.

Materialization summary result:

```text
visitor faster than reference-result: 11 of 13 workloads
visitor equal to reference-result: 1 of 13 workloads
visitor slower than reference-result: 1 of 13 workloads
large_full_dataset_range reference-result: 11.656us
large_full_dataset_range reference-visitor: 5.625us
large_full_dataset_range visitor advantage: 6.031us
```

The equal and slower rows were small absolute differences:

```text
large_empty_far_range: equal at 22ns
large_cluster_range_008: 20ns slower
```

Conclusion:

```text
reference visitor reduces reference-result output overhead when references must be delivered
large output cardinality makes the visitor advantage easier to observe
small selective rows can remain noise-level or mixed
count-only remains the resultless floor when only cardinality is needed
```

This evidence should not be interpreted as a pruning improvement. It is result-delivery evidence over exact matches after the normal FSE execution pipeline has already selected and evaluated candidates.

## Row-view materialization trial conclusion

The borrowed row-view API was tested through repeated materialization trials after row-view timing was added to the materialization summary.

The small run used:

```text
label: run003-r01-row-view-materialization-trials-small
dataset: small
trials: 5
iterations: 1000
agreement failures: 0
```

Small result:

```text
row-view beat reference-result on the selective cluster workloads
row-view did not beat reference-visitor on any workload
full_dataset_range row-view timing was worse than reference-result and reference-visitor
```

The large run used:

```text
label: run003-r01-row-view-materialization-trials-large
dataset: large
max depth: 16
trials: 5
iterations: 1000
agreement failures: 0
```

Large result:

```text
row-view did not beat reference-visitor on any matched workload
large_cross_cluster_boundary: 532.2ns row-view versus 381.0ns reference-visitor
large_full_dataset_range: 18.6414us row-view versus 3.9052us reference-visitor
```

Conclusion:

```text
row-view is semantically valid and useful as a borrowed API shape
row-view is not the next performance direction
reference-visitor remains the better streaming exact-match delivery path
do not expand row-view into reference-list or batch APIs for performance without new evidence
do not use row-view materialization evidence to justify SIMD reconstruction work
```

## Future optimization direction

The next meaningful optimization direction is a result representation or query API decision.

Completed output-contract directions:

```text
count-only query API
exists-any query API
visitor/callback-based query API
borrowed row-view result API
materialization-specific repeated trial workflow
```

Open result-representation directions:

```text
durable existence comparison track, if evidence requires it
public output-contract documentation
fallible query API variants
```

Future result-representation work should be treated as an intentional API or output-contract change, not incidental hot-path cleanup. Row-view remains valid API surface, but current evidence does not justify expanding it as a performance project.

## Acceptance rule for result-representation work

Acceptance rule for result-representation work:

```text
validation passes
matched record counts remain exact
owned-result query behavior remains unchanged unless the commit explicitly changes the API
count-only or resultless APIs must be tested separately from owned-result APIs
large dataset target workload should remain valid at max depth 16
small dataset target workload should remain valid at max depth 8
performance claims must use repeated-trial evidence
```

## Current diagnostic posture

Current benchmark interpretation:

```text
FSE pruning is doing its job on the target workloads
small boundary workload is near the fixed-cost floor under owned-result execution
large target confirms result materialization dominates retained execution
count-only query mode is a valid separate output contract
existence query mode is a valid separate exact non-empty-result output contract
reference visitor is a valid separate exact reference-delivery output contract
row-view is a valid borrowed row-delivery output contract but not the current performance direction
future traversal or boundary-specific optimizations need new evidence before being prioritized
```

Current do-not-overinterpret list:

```text
single-run timing movement
debug-only retained diagnostics as public API timing
full-selectivity count-only speedups as traversal improvements
small tiny-query behavior as large-scale behavior
timing movement from reporting-only commits
```

Current useful evidence sources:

```text
owned-result low-gap repeated trials
target workload repeated trials
count-only workload summary
count-only workload comparison
existence timing summary
reference visitor materialization summary
repeated materialization trial summaries
debug diagnostics for explanation only
```
