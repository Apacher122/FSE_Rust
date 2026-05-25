param(
  [string]$Label = "local",
  [string]$PreviousTargetSummaryCsv,
  [string]$CurrentTargetSummaryCsv,
  [string]$OutputDir = "benchmark_artifacts",
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

. $BenchmarkCsvLibrary

if ([string]::IsNullOrWhiteSpace($PreviousTargetSummaryCsv)) {
  throw "`-PreviousTargetSummaryCsv` is required"
}

if ([string]::IsNullOrWhiteSpace($CurrentTargetSummaryCsv)) {
  throw "`-CurrentTargetSummaryCsv` is required"
}

if (!(Test-Path -LiteralPath $PreviousTargetSummaryCsv)) {
  throw "previous target workload summary CSV was not found: $PreviousTargetSummaryCsv"
}

if (!(Test-Path -LiteralPath $CurrentTargetSummaryCsv)) {
  throw "current target workload summary CSV was not found: $CurrentTargetSummaryCsv"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$ComparisonCsvPath = Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv"
$ComparisonNotesPath = Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt"

$TreeBaselines = @("kd_tree", "r_tree")

function Get-SummaryRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName
  )

  return $Rows | Where-Object { $_.baseline_name -eq $BaselineName } | Select-Object -First 1
}

function New-ComparisonRow {
  param(
    [string]$BaselineName,
    [object]$PreviousRow,
    [object]$CurrentRow
  )

  $PreviousWorkloadName = Get-TextField -Row $PreviousRow -FieldName "workload_name"
  $CurrentWorkloadName = Get-TextField -Row $CurrentRow -FieldName "workload_name"

  $PreviousMeanTiming = Get-Metric -Row $PreviousRow -FieldName "mean_timing_ratio"
  $CurrentMeanTiming = Get-Metric -Row $CurrentRow -FieldName "mean_timing_ratio"
  $MeanTimingDelta = Get-Delta -CurrentValue $CurrentMeanTiming -PreviousValue $PreviousMeanTiming
  $TimingClassification = Get-Classification -Delta $MeanTimingDelta -Threshold $NoiseThreshold

  $PreviousRange = Get-Metric -Row $PreviousRow -FieldName "range_timing_ratio"
  $CurrentRange = Get-Metric -Row $CurrentRow -FieldName "range_timing_ratio"
  $RangeDelta = Get-Delta -CurrentValue $CurrentRange -PreviousValue $PreviousRange

  $PreviousStdDev = Get-Metric -Row $PreviousRow -FieldName "sample_stddev_timing_ratio"
  $CurrentStdDev = Get-Metric -Row $CurrentRow -FieldName "sample_stddev_timing_ratio"
  $StdDevDelta = Get-Delta -CurrentValue $CurrentStdDev -PreviousValue $PreviousStdDev

  $PreviousBaselineAverage = Get-Metric -Row $PreviousRow -FieldName "mean_baseline_average_elapsed_ns"
  $CurrentBaselineAverage = Get-Metric -Row $CurrentRow -FieldName "mean_baseline_average_elapsed_ns"
  $BaselineAverageDelta = Get-Delta -CurrentValue $CurrentBaselineAverage -PreviousValue $PreviousBaselineAverage

  $PreviousFseAverage = Get-Metric -Row $PreviousRow -FieldName "mean_fse_average_elapsed_ns"
  $CurrentFseAverage = Get-Metric -Row $CurrentRow -FieldName "mean_fse_average_elapsed_ns"
  $FseAverageDelta = Get-Delta -CurrentValue $CurrentFseAverage -PreviousValue $PreviousFseAverage

  $PreviousCountOnlyAverage = Get-Metric -Row $PreviousRow -FieldName "mean_count_only_average_elapsed_ns"
  $CurrentCountOnlyAverage = Get-Metric -Row $CurrentRow -FieldName "mean_count_only_average_elapsed_ns"
  $CountOnlyAverageDelta = Get-Delta -CurrentValue $CurrentCountOnlyAverage -PreviousValue $PreviousCountOnlyAverage

  $PreviousOwnedResultOverhead = Get-Metric -Row $PreviousRow -FieldName "mean_owned_result_overhead_ns"
  $CurrentOwnedResultOverhead = Get-Metric -Row $CurrentRow -FieldName "mean_owned_result_overhead_ns"
  $OwnedResultOverheadDelta = Get-Delta -CurrentValue $CurrentOwnedResultOverhead -PreviousValue $PreviousOwnedResultOverhead

  $PreviousCountOnlySpeedup = Get-Metric -Row $PreviousRow -FieldName "mean_count_only_speedup_ratio"
  $CurrentCountOnlySpeedup = Get-Metric -Row $CurrentRow -FieldName "mean_count_only_speedup_ratio"
  $CountOnlySpeedupDelta = Get-Delta -CurrentValue $CurrentCountOnlySpeedup -PreviousValue $PreviousCountOnlySpeedup

  $PreviousCountOnlyStatsAgreeCount = Get-Metric -Row $PreviousRow -FieldName "count_only_stats_agree_count"
  $CurrentCountOnlyStatsAgreeCount = Get-Metric -Row $CurrentRow -FieldName "count_only_stats_agree_count"
  $CountOnlyStatsAgreeCountDelta = Get-Delta -CurrentValue $CurrentCountOnlyStatsAgreeCount -PreviousValue $PreviousCountOnlyStatsAgreeCount

  $PreviousAllCountOnlyStatsMatchOwned = Get-LowerTextField -Row $PreviousRow -FieldName "all_count_only_stats_match_owned"
  $CurrentAllCountOnlyStatsMatchOwned = Get-LowerTextField -Row $CurrentRow -FieldName "all_count_only_stats_match_owned"

  $PreviousReferenceAverage = Get-Metric -Row $PreviousRow -FieldName "mean_reference_average_elapsed_ns"
  $CurrentReferenceAverage = Get-Metric -Row $CurrentRow -FieldName "mean_reference_average_elapsed_ns"
  $ReferenceAverageDelta = Get-Delta -CurrentValue $CurrentReferenceAverage -PreviousValue $PreviousReferenceAverage

  $PreviousOwnedVsReferenceOverhead = Get-Metric -Row $PreviousRow -FieldName "mean_owned_vs_reference_overhead_ns"
  $CurrentOwnedVsReferenceOverhead = Get-Metric -Row $CurrentRow -FieldName "mean_owned_vs_reference_overhead_ns"
  $OwnedVsReferenceOverheadDelta = Get-Delta -CurrentValue $CurrentOwnedVsReferenceOverhead -PreviousValue $PreviousOwnedVsReferenceOverhead

  $PreviousReferenceResultSpeedup = Get-Metric -Row $PreviousRow -FieldName "mean_reference_result_speedup_ratio"
  $CurrentReferenceResultSpeedup = Get-Metric -Row $CurrentRow -FieldName "mean_reference_result_speedup_ratio"
  $ReferenceResultSpeedupDelta = Get-Delta -CurrentValue $CurrentReferenceResultSpeedup -PreviousValue $PreviousReferenceResultSpeedup

  $PreviousReferenceStatsAgreeCount = Get-Metric -Row $PreviousRow -FieldName "reference_stats_agree_count"
  $CurrentReferenceStatsAgreeCount = Get-Metric -Row $CurrentRow -FieldName "reference_stats_agree_count"
  $ReferenceStatsAgreeCountDelta = Get-Delta -CurrentValue $CurrentReferenceStatsAgreeCount -PreviousValue $PreviousReferenceStatsAgreeCount

  $PreviousAllReferenceStatsMatchCountOnly = Get-LowerTextField -Row $PreviousRow -FieldName "all_reference_stats_match_count_only"
  $CurrentAllReferenceStatsMatchCountOnly = Get-LowerTextField -Row $CurrentRow -FieldName "all_reference_stats_match_count_only"

  $PreviousReferenceVisitedNodes = Get-Metric -Row $PreviousRow -FieldName "mean_reference_visited_nodes"
  $CurrentReferenceVisitedNodes = Get-Metric -Row $CurrentRow -FieldName "mean_reference_visited_nodes"
  $ReferenceVisitedNodeDelta = Get-Delta -CurrentValue $CurrentReferenceVisitedNodes -PreviousValue $PreviousReferenceVisitedNodes

  $PreviousReferenceRetainedLeaves = Get-Metric -Row $PreviousRow -FieldName "mean_reference_retained_leaves"
  $CurrentReferenceRetainedLeaves = Get-Metric -Row $CurrentRow -FieldName "mean_reference_retained_leaves"
  $ReferenceRetainedLeafDelta = Get-Delta -CurrentValue $CurrentReferenceRetainedLeaves -PreviousValue $PreviousReferenceRetainedLeaves

  $PreviousReferenceReconstructedRecords = Get-Metric -Row $PreviousRow -FieldName "mean_reference_reconstructed_records"
  $CurrentReferenceReconstructedRecords = Get-Metric -Row $CurrentRow -FieldName "mean_reference_reconstructed_records"
  $ReferenceReconstructedRecordDelta = Get-Delta -CurrentValue $CurrentReferenceReconstructedRecords -PreviousValue $PreviousReferenceReconstructedRecords

  $PreviousReferenceMatchedRecords = Get-Metric -Row $PreviousRow -FieldName "mean_reference_matched_records"
  $CurrentReferenceMatchedRecords = Get-Metric -Row $CurrentRow -FieldName "mean_reference_matched_records"
  $ReferenceMatchedRecordDelta = Get-Delta -CurrentValue $CurrentReferenceMatchedRecords -PreviousValue $PreviousReferenceMatchedRecords

  $PreviousVisitedNodes = Get-Metric -Row $PreviousRow -FieldName "mean_fse_visited_nodes"
  $CurrentVisitedNodes = Get-Metric -Row $CurrentRow -FieldName "mean_fse_visited_nodes"
  $VisitedNodeDelta = Get-Delta -CurrentValue $CurrentVisitedNodes -PreviousValue $PreviousVisitedNodes

  $PreviousRetainedLeaves = Get-Metric -Row $PreviousRow -FieldName "mean_fse_retained_leaves"
  $CurrentRetainedLeaves = Get-Metric -Row $CurrentRow -FieldName "mean_fse_retained_leaves"
  $RetainedLeafDelta = Get-Delta -CurrentValue $CurrentRetainedLeaves -PreviousValue $PreviousRetainedLeaves

  $PreviousReconstructedRecords = Get-Metric -Row $PreviousRow -FieldName "mean_fse_reconstructed_records"
  $CurrentReconstructedRecords = Get-Metric -Row $CurrentRow -FieldName "mean_fse_reconstructed_records"
  $ReconstructedRecordDelta = Get-Delta -CurrentValue $CurrentReconstructedRecords -PreviousValue $PreviousReconstructedRecords

  $PreviousMatchedRecords = Get-Metric -Row $PreviousRow -FieldName "mean_fse_matched_records"
  $CurrentMatchedRecords = Get-Metric -Row $CurrentRow -FieldName "mean_fse_matched_records"
  $MatchedRecordDelta = Get-Delta -CurrentValue $CurrentMatchedRecords -PreviousValue $PreviousMatchedRecords

  $PreviousBaselineEvaluated = Get-Metric -Row $PreviousRow -FieldName "mean_baseline_evaluated_records"
  $CurrentBaselineEvaluated = Get-Metric -Row $CurrentRow -FieldName "mean_baseline_evaluated_records"
  $BaselineEvaluatedDelta = Get-Delta -CurrentValue $CurrentBaselineEvaluated -PreviousValue $PreviousBaselineEvaluated

  $PreviousBaselineMatched = Get-Metric -Row $PreviousRow -FieldName "mean_baseline_matched_records"
  $CurrentBaselineMatched = Get-Metric -Row $CurrentRow -FieldName "mean_baseline_matched_records"
  $BaselineMatchedDelta = Get-Delta -CurrentValue $CurrentBaselineMatched -PreviousValue $PreviousBaselineMatched

  $PreviousCandidateRatio = Get-Metric -Row $PreviousRow -FieldName "mean_candidate_ratio"
  $CurrentCandidateRatio = Get-Metric -Row $CurrentRow -FieldName "mean_candidate_ratio"
  $CandidateRatioDelta = Get-Delta -CurrentValue $CurrentCandidateRatio -PreviousValue $PreviousCandidateRatio

  $PreviousAvoidanceRatio = Get-Metric -Row $PreviousRow -FieldName "mean_reconstruction_avoidance_ratio"
  $CurrentAvoidanceRatio = Get-Metric -Row $CurrentRow -FieldName "mean_reconstruction_avoidance_ratio"
  $AvoidanceRatioDelta = Get-Delta -CurrentValue $CurrentAvoidanceRatio -PreviousValue $PreviousAvoidanceRatio

  $PreviousNodesPerRecord = Get-Metric -Row $PreviousRow -FieldName "mean_nodes_per_record"
  $CurrentNodesPerRecord = Get-Metric -Row $CurrentRow -FieldName "mean_nodes_per_record"
  $NodesPerRecordDelta = Get-Delta -CurrentValue $CurrentNodesPerRecord -PreviousValue $PreviousNodesPerRecord

  return [PSCustomObject]@{
    BaselineName                            = $BaselineName
    PreviousWorkloadName                    = $PreviousWorkloadName
    CurrentWorkloadName                     = $CurrentWorkloadName
    PreviousMeanTiming                      = $PreviousMeanTiming
    CurrentMeanTiming                       = $CurrentMeanTiming
    MeanTimingDelta                         = $MeanTimingDelta
    TimingClassification                    = $TimingClassification
    PreviousRange                           = $PreviousRange
    CurrentRange                            = $CurrentRange
    RangeDelta                              = $RangeDelta
    PreviousStdDev                          = $PreviousStdDev
    CurrentStdDev                           = $CurrentStdDev
    StdDevDelta                             = $StdDevDelta
    PreviousBaselineAverage                 = $PreviousBaselineAverage
    CurrentBaselineAverage                  = $CurrentBaselineAverage
    BaselineAverageDelta                    = $BaselineAverageDelta
    PreviousFseAverage                      = $PreviousFseAverage
    CurrentFseAverage                       = $CurrentFseAverage
    FseAverageDelta                         = $FseAverageDelta
    PreviousCountOnlyAverage                = $PreviousCountOnlyAverage
    CurrentCountOnlyAverage                 = $CurrentCountOnlyAverage
    CountOnlyAverageDelta                   = $CountOnlyAverageDelta
    PreviousOwnedResultOverhead             = $PreviousOwnedResultOverhead
    CurrentOwnedResultOverhead              = $CurrentOwnedResultOverhead
    OwnedResultOverheadDelta                = $OwnedResultOverheadDelta
    PreviousCountOnlySpeedup                = $PreviousCountOnlySpeedup
    CurrentCountOnlySpeedup                 = $CurrentCountOnlySpeedup
    CountOnlySpeedupDelta                   = $CountOnlySpeedupDelta
    PreviousCountOnlyStatsAgreeCount        = $PreviousCountOnlyStatsAgreeCount
    CurrentCountOnlyStatsAgreeCount         = $CurrentCountOnlyStatsAgreeCount
    CountOnlyStatsAgreeCountDelta           = $CountOnlyStatsAgreeCountDelta
    PreviousAllCountOnlyStatsMatchOwned     = $PreviousAllCountOnlyStatsMatchOwned
    CurrentAllCountOnlyStatsMatchOwned      = $CurrentAllCountOnlyStatsMatchOwned
    PreviousReferenceAverage                = $PreviousReferenceAverage
    CurrentReferenceAverage                 = $CurrentReferenceAverage
    ReferenceAverageDelta                   = $ReferenceAverageDelta
    PreviousOwnedVsReferenceOverhead        = $PreviousOwnedVsReferenceOverhead
    CurrentOwnedVsReferenceOverhead         = $CurrentOwnedVsReferenceOverhead
    OwnedVsReferenceOverheadDelta           = $OwnedVsReferenceOverheadDelta
    PreviousReferenceResultSpeedup          = $PreviousReferenceResultSpeedup
    CurrentReferenceResultSpeedup           = $CurrentReferenceResultSpeedup
    ReferenceResultSpeedupDelta             = $ReferenceResultSpeedupDelta
    PreviousReferenceStatsAgreeCount        = $PreviousReferenceStatsAgreeCount
    CurrentReferenceStatsAgreeCount         = $CurrentReferenceStatsAgreeCount
    ReferenceStatsAgreeCountDelta           = $ReferenceStatsAgreeCountDelta
    PreviousAllReferenceStatsMatchCountOnly = $PreviousAllReferenceStatsMatchCountOnly
    CurrentAllReferenceStatsMatchCountOnly  = $CurrentAllReferenceStatsMatchCountOnly
    PreviousReferenceVisitedNodes           = $PreviousReferenceVisitedNodes
    CurrentReferenceVisitedNodes            = $CurrentReferenceVisitedNodes
    ReferenceVisitedNodeDelta               = $ReferenceVisitedNodeDelta
    PreviousReferenceRetainedLeaves         = $PreviousReferenceRetainedLeaves
    CurrentReferenceRetainedLeaves          = $CurrentReferenceRetainedLeaves
    ReferenceRetainedLeafDelta              = $ReferenceRetainedLeafDelta
    PreviousReferenceReconstructedRecords   = $PreviousReferenceReconstructedRecords
    CurrentReferenceReconstructedRecords    = $CurrentReferenceReconstructedRecords
    ReferenceReconstructedRecordDelta       = $ReferenceReconstructedRecordDelta
    PreviousReferenceMatchedRecords         = $PreviousReferenceMatchedRecords
    CurrentReferenceMatchedRecords          = $CurrentReferenceMatchedRecords
    ReferenceMatchedRecordDelta             = $ReferenceMatchedRecordDelta
    PreviousVisitedNodes                    = $PreviousVisitedNodes
    CurrentVisitedNodes                     = $CurrentVisitedNodes
    VisitedNodeDelta                        = $VisitedNodeDelta
    PreviousRetainedLeaves                  = $PreviousRetainedLeaves
    CurrentRetainedLeaves                   = $CurrentRetainedLeaves
    RetainedLeafDelta                       = $RetainedLeafDelta
    PreviousReconstructedRecords            = $PreviousReconstructedRecords
    CurrentReconstructedRecords             = $CurrentReconstructedRecords
    ReconstructedRecordDelta                = $ReconstructedRecordDelta
    PreviousMatchedRecords                  = $PreviousMatchedRecords
    CurrentMatchedRecords                   = $CurrentMatchedRecords
    MatchedRecordDelta                      = $MatchedRecordDelta
    PreviousBaselineEvaluated               = $PreviousBaselineEvaluated
    CurrentBaselineEvaluated                = $CurrentBaselineEvaluated
    BaselineEvaluatedDelta                  = $BaselineEvaluatedDelta
    PreviousBaselineMatched                 = $PreviousBaselineMatched
    CurrentBaselineMatched                  = $CurrentBaselineMatched
    BaselineMatchedDelta                    = $BaselineMatchedDelta
    PreviousCandidateRatio                  = $PreviousCandidateRatio
    CurrentCandidateRatio                   = $CurrentCandidateRatio
    CandidateRatioDelta                     = $CandidateRatioDelta
    PreviousAvoidanceRatio                  = $PreviousAvoidanceRatio
    CurrentAvoidanceRatio                   = $CurrentAvoidanceRatio
    AvoidanceRatioDelta                     = $AvoidanceRatioDelta
    PreviousNodesPerRecord                  = $PreviousNodesPerRecord
    CurrentNodesPerRecord                   = $CurrentNodesPerRecord
    NodesPerRecordDelta                     = $NodesPerRecordDelta
  }
}

$PreviousResolvedPath = (Resolve-Path -LiteralPath $PreviousTargetSummaryCsv).Path
$CurrentResolvedPath = (Resolve-Path -LiteralPath $CurrentTargetSummaryCsv).Path

$PreviousRows = @(Import-Csv -LiteralPath $PreviousResolvedPath)
$CurrentRows = @(Import-Csv -LiteralPath $CurrentResolvedPath)

$ComparisonObjects = @()

foreach ($BaselineName in $TreeBaselines) {
  $PreviousRow = Get-SummaryRow -Rows $PreviousRows -BaselineName $BaselineName
  $CurrentRow = Get-SummaryRow -Rows $CurrentRows -BaselineName $BaselineName

  $ComparisonObjects += New-ComparisonRow `
    -BaselineName $BaselineName `
    -PreviousRow $PreviousRow `
    -CurrentRow $CurrentRow
}

$ComparisonRows = @()

foreach ($Comparison in $ComparisonObjects) {
  $ComparisonRows += , @(
    $Comparison.BaselineName,
    $Comparison.PreviousWorkloadName,
    $Comparison.CurrentWorkloadName,
    $(Format-InvariantDouble -Value $Comparison.PreviousMeanTiming),
    $(Format-InvariantDouble -Value $Comparison.CurrentMeanTiming),
    $(Format-InvariantDouble -Value $Comparison.MeanTimingDelta),
    $Comparison.TimingClassification,
    $(Format-InvariantDouble -Value $Comparison.PreviousRange),
    $(Format-InvariantDouble -Value $Comparison.CurrentRange),
    $(Format-InvariantDouble -Value $Comparison.RangeDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousStdDev),
    $(Format-InvariantDouble -Value $Comparison.CurrentStdDev),
    $(Format-InvariantDouble -Value $Comparison.StdDevDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousBaselineAverage),
    $(Format-InvariantDouble -Value $Comparison.CurrentBaselineAverage),
    $(Format-InvariantDouble -Value $Comparison.BaselineAverageDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousFseAverage),
    $(Format-InvariantDouble -Value $Comparison.CurrentFseAverage),
    $(Format-InvariantDouble -Value $Comparison.FseAverageDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousCountOnlyAverage),
    $(Format-InvariantDouble -Value $Comparison.CurrentCountOnlyAverage),
    $(Format-InvariantDouble -Value $Comparison.CountOnlyAverageDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousOwnedResultOverhead),
    $(Format-InvariantDouble -Value $Comparison.CurrentOwnedResultOverhead),
    $(Format-InvariantDouble -Value $Comparison.OwnedResultOverheadDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousCountOnlySpeedup),
    $(Format-InvariantDouble -Value $Comparison.CurrentCountOnlySpeedup),
    $(Format-InvariantDouble -Value $Comparison.CountOnlySpeedupDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousCountOnlyStatsAgreeCount),
    $(Format-InvariantDouble -Value $Comparison.CurrentCountOnlyStatsAgreeCount),
    $(Format-InvariantDouble -Value $Comparison.CountOnlyStatsAgreeCountDelta),
    $Comparison.PreviousAllCountOnlyStatsMatchOwned,
    $Comparison.CurrentAllCountOnlyStatsMatchOwned,
    $(Format-InvariantDouble -Value $Comparison.PreviousReferenceAverage),
    $(Format-InvariantDouble -Value $Comparison.CurrentReferenceAverage),
    $(Format-InvariantDouble -Value $Comparison.ReferenceAverageDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousOwnedVsReferenceOverhead),
    $(Format-InvariantDouble -Value $Comparison.CurrentOwnedVsReferenceOverhead),
    $(Format-InvariantDouble -Value $Comparison.OwnedVsReferenceOverheadDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousReferenceResultSpeedup),
    $(Format-InvariantDouble -Value $Comparison.CurrentReferenceResultSpeedup),
    $(Format-InvariantDouble -Value $Comparison.ReferenceResultSpeedupDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousReferenceStatsAgreeCount),
    $(Format-InvariantDouble -Value $Comparison.CurrentReferenceStatsAgreeCount),
    $(Format-InvariantDouble -Value $Comparison.ReferenceStatsAgreeCountDelta),
    $Comparison.PreviousAllReferenceStatsMatchCountOnly,
    $Comparison.CurrentAllReferenceStatsMatchCountOnly,
    $(Format-InvariantDouble -Value $Comparison.PreviousReferenceVisitedNodes),
    $(Format-InvariantDouble -Value $Comparison.CurrentReferenceVisitedNodes),
    $(Format-InvariantDouble -Value $Comparison.ReferenceVisitedNodeDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousReferenceRetainedLeaves),
    $(Format-InvariantDouble -Value $Comparison.CurrentReferenceRetainedLeaves),
    $(Format-InvariantDouble -Value $Comparison.ReferenceRetainedLeafDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousReferenceReconstructedRecords),
    $(Format-InvariantDouble -Value $Comparison.CurrentReferenceReconstructedRecords),
    $(Format-InvariantDouble -Value $Comparison.ReferenceReconstructedRecordDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousReferenceMatchedRecords),
    $(Format-InvariantDouble -Value $Comparison.CurrentReferenceMatchedRecords),
    $(Format-InvariantDouble -Value $Comparison.ReferenceMatchedRecordDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousVisitedNodes),
    $(Format-InvariantDouble -Value $Comparison.CurrentVisitedNodes),
    $(Format-InvariantDouble -Value $Comparison.VisitedNodeDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousRetainedLeaves),
    $(Format-InvariantDouble -Value $Comparison.CurrentRetainedLeaves),
    $(Format-InvariantDouble -Value $Comparison.RetainedLeafDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousReconstructedRecords),
    $(Format-InvariantDouble -Value $Comparison.CurrentReconstructedRecords),
    $(Format-InvariantDouble -Value $Comparison.ReconstructedRecordDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousMatchedRecords),
    $(Format-InvariantDouble -Value $Comparison.CurrentMatchedRecords),
    $(Format-InvariantDouble -Value $Comparison.MatchedRecordDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousBaselineEvaluated),
    $(Format-InvariantDouble -Value $Comparison.CurrentBaselineEvaluated),
    $(Format-InvariantDouble -Value $Comparison.BaselineEvaluatedDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousBaselineMatched),
    $(Format-InvariantDouble -Value $Comparison.CurrentBaselineMatched),
    $(Format-InvariantDouble -Value $Comparison.BaselineMatchedDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousCandidateRatio),
    $(Format-InvariantDouble -Value $Comparison.CurrentCandidateRatio),
    $(Format-InvariantDouble -Value $Comparison.CandidateRatioDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousAvoidanceRatio),
    $(Format-InvariantDouble -Value $Comparison.CurrentAvoidanceRatio),
    $(Format-InvariantDouble -Value $Comparison.AvoidanceRatioDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousNodesPerRecord),
    $(Format-InvariantDouble -Value $Comparison.CurrentNodesPerRecord),
    $(Format-InvariantDouble -Value $Comparison.NodesPerRecordDelta)
  )
}

Write-CsvDocument `
  -Path $ComparisonCsvPath `
  -Header @(
  "baseline_name",
  "previous_workload_name",
  "current_workload_name",
  "previous_mean_timing_ratio",
  "current_mean_timing_ratio",
  "mean_timing_delta",
  "timing_classification",
  "previous_range_timing_ratio",
  "current_range_timing_ratio",
  "range_timing_delta",
  "previous_sample_stddev_timing_ratio",
  "current_sample_stddev_timing_ratio",
  "sample_stddev_timing_delta",
  "previous_mean_baseline_average_elapsed_ns",
  "current_mean_baseline_average_elapsed_ns",
  "mean_baseline_average_elapsed_ns_delta",
  "previous_mean_fse_average_elapsed_ns",
  "current_mean_fse_average_elapsed_ns",
  "mean_fse_average_elapsed_ns_delta",
  "previous_mean_count_only_average_elapsed_ns",
  "current_mean_count_only_average_elapsed_ns",
  "mean_count_only_average_elapsed_ns_delta",
  "previous_mean_owned_result_overhead_ns",
  "current_mean_owned_result_overhead_ns",
  "mean_owned_result_overhead_ns_delta",
  "previous_mean_count_only_speedup_ratio",
  "current_mean_count_only_speedup_ratio",
  "mean_count_only_speedup_ratio_delta",
  "previous_count_only_stats_agree_count",
  "current_count_only_stats_agree_count",
  "count_only_stats_agree_count_delta",
  "previous_all_count_only_stats_match_owned",
  "current_all_count_only_stats_match_owned",
  "previous_mean_reference_average_elapsed_ns",
  "current_mean_reference_average_elapsed_ns",
  "mean_reference_average_elapsed_ns_delta",
  "previous_mean_owned_vs_reference_overhead_ns",
  "current_mean_owned_vs_reference_overhead_ns",
  "mean_owned_vs_reference_overhead_ns_delta",
  "previous_mean_reference_result_speedup_ratio",
  "current_mean_reference_result_speedup_ratio",
  "mean_reference_result_speedup_ratio_delta",
  "previous_reference_stats_agree_count",
  "current_reference_stats_agree_count",
  "reference_stats_agree_count_delta",
  "previous_all_reference_stats_match_count_only",
  "current_all_reference_stats_match_count_only",
  "previous_mean_reference_visited_nodes",
  "current_mean_reference_visited_nodes",
  "mean_reference_visited_nodes_delta",
  "previous_mean_reference_retained_leaves",
  "current_mean_reference_retained_leaves",
  "mean_reference_retained_leaves_delta",
  "previous_mean_reference_reconstructed_records",
  "current_mean_reference_reconstructed_records",
  "mean_reference_reconstructed_records_delta",
  "previous_mean_reference_matched_records",
  "current_mean_reference_matched_records",
  "mean_reference_matched_records_delta",
  "previous_mean_fse_visited_nodes",
  "current_mean_fse_visited_nodes",
  "mean_fse_visited_nodes_delta",
  "previous_mean_fse_retained_leaves",
  "current_mean_fse_retained_leaves",
  "mean_fse_retained_leaves_delta",
  "previous_mean_fse_reconstructed_records",
  "current_mean_fse_reconstructed_records",
  "mean_fse_reconstructed_records_delta",
  "previous_mean_fse_matched_records",
  "current_mean_fse_matched_records",
  "mean_fse_matched_records_delta",
  "previous_mean_baseline_evaluated_records",
  "current_mean_baseline_evaluated_records",
  "mean_baseline_evaluated_records_delta",
  "previous_mean_baseline_matched_records",
  "current_mean_baseline_matched_records",
  "mean_baseline_matched_records_delta",
  "previous_mean_candidate_ratio",
  "current_mean_candidate_ratio",
  "mean_candidate_ratio_delta",
  "previous_mean_reconstruction_avoidance_ratio",
  "current_mean_reconstruction_avoidance_ratio",
  "mean_reconstruction_avoidance_ratio_delta",
  "previous_mean_nodes_per_record",
  "current_mean_nodes_per_record",
  "mean_nodes_per_record_delta"
) `
  -Rows $ComparisonRows

Set-Utf8Text -Path $ComparisonNotesPath -Text "Target workload trial comparison`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "================================`r`n`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous target summary: $PreviousTargetSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous resolved path: $PreviousResolvedPath`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current target summary: $CurrentTargetSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current resolved path: $CurrentResolvedPath`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Timing ratio meaning: baseline elapsed / FSE elapsed; higher is better for FSE.`r`n"

foreach ($Comparison in $ComparisonObjects) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n$($Comparison.BaselineName)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text ("-" * $Comparison.BaselineName.Length)
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous workload: $($Comparison.PreviousWorkloadName)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current workload: $($Comparison.CurrentWorkloadName)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous mean timing: $(Format-InvariantDouble -Value $Comparison.PreviousMeanTiming)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current mean timing: $(Format-InvariantDouble -Value $Comparison.CurrentMeanTiming)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean timing delta: $(Format-InvariantDouble -Value $Comparison.MeanTimingDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "classification: $($Comparison.TimingClassification)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "range delta: $(Format-InvariantDouble -Value $Comparison.RangeDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "sample stddev delta: $(Format-InvariantDouble -Value $Comparison.StdDevDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean baseline average ns delta: $(Format-InvariantDouble -Value $Comparison.BaselineAverageDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE owned-result average ns delta: $(Format-InvariantDouble -Value $Comparison.FseAverageDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean count-only average ns delta: $(Format-InvariantDouble -Value $Comparison.CountOnlyAverageDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean owned-result overhead ns delta: $(Format-InvariantDouble -Value $Comparison.OwnedResultOverheadDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean count-only speedup ratio delta: $(Format-InvariantDouble -Value $Comparison.CountOnlySpeedupDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "count-only stats agree count delta: $(Format-InvariantDouble -Value $Comparison.CountOnlyStatsAgreeCountDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous all count-only stats match owned: $($Comparison.PreviousAllCountOnlyStatsMatchOwned)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current all count-only stats match owned: $($Comparison.CurrentAllCountOnlyStatsMatchOwned)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean reference-result average ns delta: $(Format-InvariantDouble -Value $Comparison.ReferenceAverageDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean owned-vs-reference overhead ns delta: $(Format-InvariantDouble -Value $Comparison.OwnedVsReferenceOverheadDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean reference-result speedup ratio delta: $(Format-InvariantDouble -Value $Comparison.ReferenceResultSpeedupDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "reference stats agree count delta: $(Format-InvariantDouble -Value $Comparison.ReferenceStatsAgreeCountDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous all reference stats match count-only: $($Comparison.PreviousAllReferenceStatsMatchCountOnly)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current all reference stats match count-only: $($Comparison.CurrentAllReferenceStatsMatchCountOnly)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean reference visited nodes delta: $(Format-InvariantDouble -Value $Comparison.ReferenceVisitedNodeDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean reference retained leaves delta: $(Format-InvariantDouble -Value $Comparison.ReferenceRetainedLeafDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean reference reconstructed records delta: $(Format-InvariantDouble -Value $Comparison.ReferenceReconstructedRecordDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean reference matched records delta: $(Format-InvariantDouble -Value $Comparison.ReferenceMatchedRecordDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE visited nodes delta: $(Format-InvariantDouble -Value $Comparison.VisitedNodeDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE retained leaves delta: $(Format-InvariantDouble -Value $Comparison.RetainedLeafDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE reconstructed records delta: $(Format-InvariantDouble -Value $Comparison.ReconstructedRecordDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE matched records delta: $(Format-InvariantDouble -Value $Comparison.MatchedRecordDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean baseline evaluated records delta: $(Format-InvariantDouble -Value $Comparison.BaselineEvaluatedDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean baseline matched records delta: $(Format-InvariantDouble -Value $Comparison.BaselineMatchedDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean candidate ratio delta: $(Format-InvariantDouble -Value $Comparison.CandidateRatioDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean reconstruction avoidance delta: $(Format-InvariantDouble -Value $Comparison.AvoidanceRatioDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean nodes per record delta: $(Format-InvariantDouble -Value $Comparison.NodesPerRecordDelta)`r`n"
}

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Use this comparison when a performance commit intentionally targets a specific workload.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "A target workload improvement is stronger when timing improves beyond the noise threshold without increasing visited nodes, reconstructed records, candidate ratio, or matched-record differences.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Owned-result overhead deltas help identify whether a change improved materialized row-return cost rather than pruning alone.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Reference-result overhead deltas help identify whether a change improved exact leaf/row-reference return cost before owned Vector materialization.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Count-only timing must only be interpreted when previous and current all-count-only-stats-match-owned values are true.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Reference-result timing must only be interpreted when previous and current all-reference-stats-match-count-only values are true.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "This comparison remains useful even if the target workload stops being the weakest workload after an optimization.`r`n"

Write-Host ""
Write-Host "Target workload comparison artifacts written:"
Write-Host "  $ComparisonCsvPath"
Write-Host "  $ComparisonNotesPath"