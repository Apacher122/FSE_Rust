param(
  [string]$Label = "local",
  [string]$InputLabel = "",
  [string]$TargetWorkloadName,
  [string]$OutputDir = "benchmark_artifacts"
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"
$BenchmarkComparisonLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-comparison.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkComparisonLibrary)) {
  throw "benchmark comparison helper library was not found: $BenchmarkComparisonLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkComparisonLibrary

if ([string]::IsNullOrWhiteSpace($Label)) {
  throw "-Label cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($InputLabel)) {
  $InputLabel = $Label
}

if ([string]::IsNullOrWhiteSpace($TargetWorkloadName)) {
  throw "-TargetWorkloadName cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
  throw "-OutputDir cannot be empty"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TrialInputDir = Join-Path $OutputDir "low-gap-trials-$InputLabel"

if (!(Test-Path -LiteralPath $TrialInputDir)) {
  throw "low-gap trial input directory was not found: $TrialInputDir"
}

$TrialWorkloadCsvPaths = @(
  Get-ChildItem `
    -LiteralPath $TrialInputDir `
    -Filter "workloads-$InputLabel-trial-*.csv" `
    -File |
  Sort-Object Name |
  ForEach-Object { $_.FullName }
)

if ($TrialWorkloadCsvPaths.Count -eq 0) {
  throw "no workload trial CSV files were found in $TrialInputDir"
}

$DetailCsvPath = Join-Path $OutputDir "target-workload-trial-details-$Label.csv"
$SummaryCsvPath = Join-Path $OutputDir "target-workload-trial-summary-$Label.csv"
$NotesPath = Join-Path $OutputDir "target-workload-trial-notes-$Label.txt"

$TreeBaselines = Get-BenchmarkTreeBaselineNames

function Get-TargetTrialNumber {
  param(
    [string]$Path,
    [int]$FallbackTrialNumber
  )

  $FileName = Split-Path -Leaf $Path
  $Match = [Regex]::Match($FileName, "trial-(\d+)\.csv$")

  if ($Match.Success) {
    return [int]$Match.Groups[1].Value
  }

  return $FallbackTrialNumber
}

function Get-RatioOrNull {
  param(
    [object]$Numerator,
    [object]$Denominator
  )

  if ($null -eq $Numerator -or $null -eq $Denominator) {
    return $null
  }

  $NumeratorValue = [double]$Numerator
  $DenominatorValue = [double]$Denominator

  if ($DenominatorValue -eq 0.0) {
    return $null
  }

  return $NumeratorValue / $DenominatorValue
}

function Get-TargetMetricValues {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return @(
    $Rows |
    Where-Object { $null -ne $_.$PropertyName } |
    ForEach-Object { [double]$_.$PropertyName }
  )
}

function Get-TargetMean {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return Get-Mean -Values (Get-TargetMetricValues -Rows $Rows -PropertyName $PropertyName)
}

function Get-TargetMin {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return Get-Minimum -Values (Get-TargetMetricValues -Rows $Rows -PropertyName $PropertyName)
}

function Get-TargetMax {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return Get-Maximum -Values (Get-TargetMetricValues -Rows $Rows -PropertyName $PropertyName)
}

function Get-TargetStdDev {
  param(
    [object[]]$Rows,
    [string]$PropertyName,
    [object]$Mean
  )

  return Get-SampleStdDev `
    -Values (Get-TargetMetricValues -Rows $Rows -PropertyName $PropertyName) `
    -Mean $Mean
}

function Get-BooleanTrueCount {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return @(
    $Rows |
    Where-Object {
      $null -ne $_.$PropertyName -and $_.$PropertyName.ToString().Trim().ToLowerInvariant() -eq "true"
    }
  ).Count
}

function Get-AllBooleanRowsTrue {
  param(
    [int]$TrialCount,
    [int]$TrueCount
  )

  return ($TrialCount -gt 0 -and $TrueCount -eq $TrialCount)
}

function Get-TargetWorkloadRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName,
    [string]$WorkloadName
  )

  return $Rows |
  Where-Object {
    $_.baseline_name -eq $BaselineName -and $_.workload_name -eq $WorkloadName
  } |
  Select-Object -First 1
}

function New-MissingTargetDetailRow {
  param(
    [int]$TrialNumber,
    [string]$BaselineName,
    [string]$SourceCsv
  )

  return [PSCustomObject]@{
    Trial                             = $TrialNumber
    BaselineName                      = $BaselineName
    WorkloadName                      = $TargetWorkloadName
    AverageTimingRatio                = $null
    BaselineAverageElapsedNs          = $null
    FseAverageElapsedNs               = $null
    CountOnlyAverageElapsedNs         = $null
    OwnedResultOverheadNs             = $null
    CountOnlySpeedupRatio             = $null
    CountOnlyStatsMatchOwned          = "false"
    ReferenceAverageElapsedNs         = $null
    OwnedVsReferenceOverheadNs        = $null
    ReferenceResultSpeedupRatio       = $null
    ReferenceStatsMatchCountOnly      = "false"
    ReferenceVisitedNodes             = $null
    ReferenceRetainedLeaves           = $null
    ReferenceReconstructedRecords     = $null
    ReferenceMatchedRecords           = $null
    ReusableOwnedAverageElapsedNs     = $null
    FreshVsReusableOwnedOverheadNs    = $null
    ReusableOwnedResultSpeedupRatio   = $null
    ReusableOwnedStatsMatchOwned      = "false"
    ReusableOwnedVisitedNodes         = $null
    ReusableOwnedRetainedLeaves       = $null
    ReusableOwnedReconstructedRecords = $null
    ReusableOwnedMatchedRecords       = $null
    FseVisitedNodes                   = $null
    FseRetainedLeaves                 = $null
    FseReconstructedRecords           = $null
    FseMatchedRecords                 = $null
    BaselineEvaluatedRecords          = $null
    BaselineMatchedRecords            = $null
    CandidateRatio                    = $null
    ReconstructionAvoidanceRatio      = $null
    NodesPerRecord                    = $null
    SourceCsv                         = $SourceCsv
    Status                            = "missing target workload"
  }
}

function New-LoadedTargetDetailRow {
  param(
    [int]$TrialNumber,
    [string]$BaselineName,
    [object]$SourceRow,
    [string]$SourceCsv
  )

  $FseVisitedNodes = Get-Metric -Row $SourceRow -FieldName "fse_visited_nodes"
  $FseReconstructedRecords = Get-Metric -Row $SourceRow -FieldName "fse_reconstructed_records"

  return [PSCustomObject]@{
    Trial                             = $TrialNumber
    BaselineName                      = $BaselineName
    WorkloadName                      = Get-TextField -Row $SourceRow -FieldName "workload_name"
    AverageTimingRatio                = Get-Metric -Row $SourceRow -FieldName "average_timing_ratio"
    BaselineAverageElapsedNs          = Get-Metric -Row $SourceRow -FieldName "baseline_average_elapsed_ns"
    FseAverageElapsedNs               = Get-Metric -Row $SourceRow -FieldName "fse_average_elapsed_ns"
    CountOnlyAverageElapsedNs         = Get-Metric -Row $SourceRow -FieldName "count_only_average_elapsed_ns"
    OwnedResultOverheadNs             = Get-Metric -Row $SourceRow -FieldName "estimated_owned_result_overhead_ns"
    CountOnlySpeedupRatio             = Get-Metric -Row $SourceRow -FieldName "count_only_speedup_ratio"
    CountOnlyStatsMatchOwned          = Get-LowerTextField -Row $SourceRow -FieldName "count_only_stats_match_owned"
    ReferenceAverageElapsedNs         = Get-Metric -Row $SourceRow -FieldName "reference_average_elapsed_ns"
    OwnedVsReferenceOverheadNs        = Get-Metric -Row $SourceRow -FieldName "estimated_owned_vs_reference_overhead_ns"
    ReferenceResultSpeedupRatio       = Get-Metric -Row $SourceRow -FieldName "reference_result_speedup_ratio"
    ReferenceStatsMatchCountOnly      = Get-LowerTextField -Row $SourceRow -FieldName "reference_stats_match_count_only"
    ReferenceVisitedNodes             = Get-Metric -Row $SourceRow -FieldName "reference_visited_nodes"
    ReferenceRetainedLeaves           = Get-Metric -Row $SourceRow -FieldName "reference_retained_leaves"
    ReferenceReconstructedRecords     = Get-Metric -Row $SourceRow -FieldName "reference_reconstructed_records"
    ReferenceMatchedRecords           = Get-Metric -Row $SourceRow -FieldName "reference_matched_records"
    ReusableOwnedAverageElapsedNs     = Get-Metric -Row $SourceRow -FieldName "reusable_owned_average_elapsed_ns"
    FreshVsReusableOwnedOverheadNs    = Get-Metric -Row $SourceRow -FieldName "estimated_fresh_vs_reusable_owned_overhead_ns"
    ReusableOwnedResultSpeedupRatio   = Get-Metric -Row $SourceRow -FieldName "reusable_owned_result_speedup_ratio"
    ReusableOwnedStatsMatchOwned      = Get-LowerTextField -Row $SourceRow -FieldName "reusable_owned_stats_match_owned"
    ReusableOwnedVisitedNodes         = Get-Metric -Row $SourceRow -FieldName "reusable_owned_visited_nodes"
    ReusableOwnedRetainedLeaves       = Get-Metric -Row $SourceRow -FieldName "reusable_owned_retained_leaves"
    ReusableOwnedReconstructedRecords = Get-Metric -Row $SourceRow -FieldName "reusable_owned_reconstructed_records"
    ReusableOwnedMatchedRecords       = Get-Metric -Row $SourceRow -FieldName "reusable_owned_matched_records"
    FseVisitedNodes                   = $FseVisitedNodes
    FseRetainedLeaves                 = Get-Metric -Row $SourceRow -FieldName "fse_retained_leaves"
    FseReconstructedRecords           = $FseReconstructedRecords
    FseMatchedRecords                 = Get-Metric -Row $SourceRow -FieldName "fse_matched_records"
    BaselineEvaluatedRecords          = Get-Metric -Row $SourceRow -FieldName "baseline_evaluated_records"
    BaselineMatchedRecords            = Get-Metric -Row $SourceRow -FieldName "baseline_matched_records"
    CandidateRatio                    = Get-Metric -Row $SourceRow -FieldName "candidate_ratio"
    ReconstructionAvoidanceRatio      = Get-Metric -Row $SourceRow -FieldName "reconstruction_avoidance_ratio"
    NodesPerRecord                    = Get-RatioOrNull -Numerator $FseVisitedNodes -Denominator $FseReconstructedRecords
    SourceCsv                         = $SourceCsv
    Status                            = "loaded"
  }
}

function New-TargetSummaryRow {
  param(
    [string]$BaselineName,
    [object[]]$DetailRows
  )

  $Rows = @(
    $DetailRows |
    Where-Object {
      $_.BaselineName -eq $BaselineName -and $_.Status -eq "loaded"
    }
  )

  $TrialCount = $Rows.Count
  $MeanTiming = Get-TargetMean -Rows $Rows -PropertyName "AverageTimingRatio"
  $MinTiming = Get-TargetMin -Rows $Rows -PropertyName "AverageTimingRatio"
  $MaxTiming = Get-TargetMax -Rows $Rows -PropertyName "AverageTimingRatio"
  $RangeTiming = $null

  if ($null -ne $MinTiming -and $null -ne $MaxTiming) {
    $RangeTiming = $MaxTiming - $MinTiming
  }

  $CountOnlyStatsAgreeCount = Get-BooleanTrueCount -Rows $Rows -PropertyName "CountOnlyStatsMatchOwned"
  $ReferenceStatsAgreeCount = Get-BooleanTrueCount -Rows $Rows -PropertyName "ReferenceStatsMatchCountOnly"
  $ReusableOwnedStatsAgreeCount = Get-BooleanTrueCount -Rows $Rows -PropertyName "ReusableOwnedStatsMatchOwned"

  return [PSCustomObject]@{
    BaselineName                          = $BaselineName
    WorkloadName                          = $TargetWorkloadName
    TrialCount                            = $TrialCount
    MeanTimingRatio                       = $MeanTiming
    MinTimingRatio                        = $MinTiming
    MaxTimingRatio                        = $MaxTiming
    RangeTimingRatio                      = $RangeTiming
    SampleStdDevTimingRatio               = Get-TargetStdDev -Rows $Rows -PropertyName "AverageTimingRatio" -Mean $MeanTiming
    MeanBaselineAverageElapsedNs          = Get-TargetMean -Rows $Rows -PropertyName "BaselineAverageElapsedNs"
    MeanFseAverageElapsedNs               = Get-TargetMean -Rows $Rows -PropertyName "FseAverageElapsedNs"
    MeanCountOnlyAverageElapsedNs         = Get-TargetMean -Rows $Rows -PropertyName "CountOnlyAverageElapsedNs"
    MeanOwnedResultOverheadNs             = Get-TargetMean -Rows $Rows -PropertyName "OwnedResultOverheadNs"
    MeanCountOnlySpeedupRatio             = Get-TargetMean -Rows $Rows -PropertyName "CountOnlySpeedupRatio"
    CountOnlyStatsAgreeCount              = $CountOnlyStatsAgreeCount
    AllCountOnlyStatsMatchOwned           = Get-AllBooleanRowsTrue -TrialCount $TrialCount -TrueCount $CountOnlyStatsAgreeCount
    MeanReferenceAverageElapsedNs         = Get-TargetMean -Rows $Rows -PropertyName "ReferenceAverageElapsedNs"
    MeanOwnedVsReferenceOverheadNs        = Get-TargetMean -Rows $Rows -PropertyName "OwnedVsReferenceOverheadNs"
    MeanReferenceResultSpeedupRatio       = Get-TargetMean -Rows $Rows -PropertyName "ReferenceResultSpeedupRatio"
    ReferenceStatsAgreeCount              = $ReferenceStatsAgreeCount
    AllReferenceStatsMatchCountOnly       = Get-AllBooleanRowsTrue -TrialCount $TrialCount -TrueCount $ReferenceStatsAgreeCount
    MeanReferenceVisitedNodes             = Get-TargetMean -Rows $Rows -PropertyName "ReferenceVisitedNodes"
    MeanReferenceRetainedLeaves           = Get-TargetMean -Rows $Rows -PropertyName "ReferenceRetainedLeaves"
    MeanReferenceReconstructedRecords     = Get-TargetMean -Rows $Rows -PropertyName "ReferenceReconstructedRecords"
    MeanReferenceMatchedRecords           = Get-TargetMean -Rows $Rows -PropertyName "ReferenceMatchedRecords"
    MeanReusableOwnedAverageElapsedNs     = Get-TargetMean -Rows $Rows -PropertyName "ReusableOwnedAverageElapsedNs"
    MeanFreshVsReusableOwnedOverheadNs    = Get-TargetMean -Rows $Rows -PropertyName "FreshVsReusableOwnedOverheadNs"
    MeanReusableOwnedResultSpeedupRatio   = Get-TargetMean -Rows $Rows -PropertyName "ReusableOwnedResultSpeedupRatio"
    ReusableOwnedStatsAgreeCount          = $ReusableOwnedStatsAgreeCount
    AllReusableOwnedStatsMatchOwned       = Get-AllBooleanRowsTrue -TrialCount $TrialCount -TrueCount $ReusableOwnedStatsAgreeCount
    MeanReusableOwnedVisitedNodes         = Get-TargetMean -Rows $Rows -PropertyName "ReusableOwnedVisitedNodes"
    MeanReusableOwnedRetainedLeaves       = Get-TargetMean -Rows $Rows -PropertyName "ReusableOwnedRetainedLeaves"
    MeanReusableOwnedReconstructedRecords = Get-TargetMean -Rows $Rows -PropertyName "ReusableOwnedReconstructedRecords"
    MeanReusableOwnedMatchedRecords       = Get-TargetMean -Rows $Rows -PropertyName "ReusableOwnedMatchedRecords"
    MeanFseVisitedNodes                   = Get-TargetMean -Rows $Rows -PropertyName "FseVisitedNodes"
    MeanFseRetainedLeaves                 = Get-TargetMean -Rows $Rows -PropertyName "FseRetainedLeaves"
    MeanFseReconstructedRecords           = Get-TargetMean -Rows $Rows -PropertyName "FseReconstructedRecords"
    MeanFseMatchedRecords                 = Get-TargetMean -Rows $Rows -PropertyName "FseMatchedRecords"
    MeanBaselineEvaluatedRecords          = Get-TargetMean -Rows $Rows -PropertyName "BaselineEvaluatedRecords"
    MeanBaselineMatchedRecords            = Get-TargetMean -Rows $Rows -PropertyName "BaselineMatchedRecords"
    MeanCandidateRatio                    = Get-TargetMean -Rows $Rows -PropertyName "CandidateRatio"
    MeanReconstructionAvoidanceRatio      = Get-TargetMean -Rows $Rows -PropertyName "ReconstructionAvoidanceRatio"
    MeanNodesPerRecord                    = Get-TargetMean -Rows $Rows -PropertyName "NodesPerRecord"
  }
}

$DetailObjects = @()
$TrialNumberFallback = 1

foreach ($TrialWorkloadCsvPath in $TrialWorkloadCsvPaths) {
  $TrialNumber = Get-TargetTrialNumber `
    -Path $TrialWorkloadCsvPath `
    -FallbackTrialNumber $TrialNumberFallback

  $Rows = @(Import-Csv -LiteralPath $TrialWorkloadCsvPath)

  foreach ($BaselineName in $TreeBaselines) {
    $TargetRow = Get-TargetWorkloadRow `
      -Rows $Rows `
      -BaselineName $BaselineName `
      -WorkloadName $TargetWorkloadName

    if ($null -eq $TargetRow) {
      $DetailObjects += New-MissingTargetDetailRow `
        -TrialNumber $TrialNumber `
        -BaselineName $BaselineName `
        -SourceCsv $TrialWorkloadCsvPath

      continue
    }

    $DetailObjects += New-LoadedTargetDetailRow `
      -TrialNumber $TrialNumber `
      -BaselineName $BaselineName `
      -SourceRow $TargetRow `
      -SourceCsv $TrialWorkloadCsvPath
  }

  $TrialNumberFallback += 1
}

$DetailRows = @()

foreach ($Row in $DetailObjects) {
  $DetailRows += , @(
    $Row.Trial,
    $Row.BaselineName,
    $Row.WorkloadName,
    $(Format-InvariantDouble -Value $Row.AverageTimingRatio),
    $(Format-InvariantDouble -Value $Row.BaselineAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.FseAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.CountOnlyAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.OwnedResultOverheadNs),
    $(Format-InvariantDouble -Value $Row.CountOnlySpeedupRatio),
    $Row.CountOnlyStatsMatchOwned,
    $(Format-InvariantDouble -Value $Row.ReferenceAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.OwnedVsReferenceOverheadNs),
    $(Format-InvariantDouble -Value $Row.ReferenceResultSpeedupRatio),
    $Row.ReferenceStatsMatchCountOnly,
    $(Format-InvariantDouble -Value $Row.ReferenceVisitedNodes),
    $(Format-InvariantDouble -Value $Row.ReferenceRetainedLeaves),
    $(Format-InvariantDouble -Value $Row.ReferenceReconstructedRecords),
    $(Format-InvariantDouble -Value $Row.ReferenceMatchedRecords),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.FreshVsReusableOwnedOverheadNs),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedResultSpeedupRatio),
    $Row.ReusableOwnedStatsMatchOwned,
    $(Format-InvariantDouble -Value $Row.ReusableOwnedVisitedNodes),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedRetainedLeaves),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedReconstructedRecords),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedMatchedRecords),
    $(Format-InvariantDouble -Value $Row.FseVisitedNodes),
    $(Format-InvariantDouble -Value $Row.FseRetainedLeaves),
    $(Format-InvariantDouble -Value $Row.FseReconstructedRecords),
    $(Format-InvariantDouble -Value $Row.FseMatchedRecords),
    $(Format-InvariantDouble -Value $Row.BaselineEvaluatedRecords),
    $(Format-InvariantDouble -Value $Row.BaselineMatchedRecords),
    $(Format-InvariantDouble -Value $Row.CandidateRatio),
    $(Format-InvariantDouble -Value $Row.ReconstructionAvoidanceRatio),
    $(Format-InvariantDouble -Value $Row.NodesPerRecord),
    $Row.Status,
    $Row.SourceCsv
  )
}

Write-CsvDocument `
  -Path $DetailCsvPath `
  -Header @(
  "trial",
  "baseline_name",
  "workload_name",
  "average_timing_ratio",
  "baseline_average_elapsed_ns",
  "fse_average_elapsed_ns",
  "count_only_average_elapsed_ns",
  "estimated_owned_result_overhead_ns",
  "count_only_speedup_ratio",
  "count_only_stats_match_owned",
  "reference_average_elapsed_ns",
  "estimated_owned_vs_reference_overhead_ns",
  "reference_result_speedup_ratio",
  "reference_stats_match_count_only",
  "reference_visited_nodes",
  "reference_retained_leaves",
  "reference_reconstructed_records",
  "reference_matched_records",
  "reusable_owned_average_elapsed_ns",
  "estimated_fresh_vs_reusable_owned_overhead_ns",
  "reusable_owned_result_speedup_ratio",
  "reusable_owned_stats_match_owned",
  "reusable_owned_visited_nodes",
  "reusable_owned_retained_leaves",
  "reusable_owned_reconstructed_records",
  "reusable_owned_matched_records",
  "fse_visited_nodes",
  "fse_retained_leaves",
  "fse_reconstructed_records",
  "fse_matched_records",
  "baseline_evaluated_records",
  "baseline_matched_records",
  "candidate_ratio",
  "reconstruction_avoidance_ratio",
  "nodes_per_record",
  "status",
  "source_csv"
) `
  -Rows $DetailRows

$SummaryObjects = @()

foreach ($BaselineName in $TreeBaselines) {
  $SummaryObjects += New-TargetSummaryRow `
    -BaselineName $BaselineName `
    -DetailRows $DetailObjects
}

$SummaryRows = @()

foreach ($Summary in $SummaryObjects) {
  $SummaryRows += , @(
    $Summary.BaselineName,
    $Summary.WorkloadName,
    $Summary.TrialCount,
    $(Format-InvariantDouble -Value $Summary.MeanTimingRatio),
    $(Format-InvariantDouble -Value $Summary.MinTimingRatio),
    $(Format-InvariantDouble -Value $Summary.MaxTimingRatio),
    $(Format-InvariantDouble -Value $Summary.RangeTimingRatio),
    $(Format-InvariantDouble -Value $Summary.SampleStdDevTimingRatio),
    $(Format-InvariantDouble -Value $Summary.MeanBaselineAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanFseAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanCountOnlyAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanOwnedResultOverheadNs),
    $(Format-InvariantDouble -Value $Summary.MeanCountOnlySpeedupRatio),
    $Summary.CountOnlyStatsAgreeCount,
    $Summary.AllCountOnlyStatsMatchOwned.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanOwnedVsReferenceOverheadNs),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceResultSpeedupRatio),
    $Summary.ReferenceStatsAgreeCount,
    $Summary.AllReferenceStatsMatchCountOnly.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceVisitedNodes),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceRetainedLeaves),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceReconstructedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanFreshVsReusableOwnedOverheadNs),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedResultSpeedupRatio),
    $Summary.ReusableOwnedStatsAgreeCount,
    $Summary.AllReusableOwnedStatsMatchOwned.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedVisitedNodes),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedRetainedLeaves),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedReconstructedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanFseVisitedNodes),
    $(Format-InvariantDouble -Value $Summary.MeanFseRetainedLeaves),
    $(Format-InvariantDouble -Value $Summary.MeanFseReconstructedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanFseMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanBaselineEvaluatedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanBaselineMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanCandidateRatio),
    $(Format-InvariantDouble -Value $Summary.MeanReconstructionAvoidanceRatio),
    $(Format-InvariantDouble -Value $Summary.MeanNodesPerRecord)
  )
}

Write-CsvDocument `
  -Path $SummaryCsvPath `
  -Header @(
  "baseline_name",
  "workload_name",
  "trial_count",
  "mean_timing_ratio",
  "min_timing_ratio",
  "max_timing_ratio",
  "range_timing_ratio",
  "sample_stddev_timing_ratio",
  "mean_baseline_average_elapsed_ns",
  "mean_fse_average_elapsed_ns",
  "mean_count_only_average_elapsed_ns",
  "mean_owned_result_overhead_ns",
  "mean_count_only_speedup_ratio",
  "count_only_stats_agree_count",
  "all_count_only_stats_match_owned",
  "mean_reference_average_elapsed_ns",
  "mean_owned_vs_reference_overhead_ns",
  "mean_reference_result_speedup_ratio",
  "reference_stats_agree_count",
  "all_reference_stats_match_count_only",
  "mean_reference_visited_nodes",
  "mean_reference_retained_leaves",
  "mean_reference_reconstructed_records",
  "mean_reference_matched_records",
  "mean_reusable_owned_average_elapsed_ns",
  "mean_fresh_vs_reusable_owned_overhead_ns",
  "mean_reusable_owned_result_speedup_ratio",
  "reusable_owned_stats_agree_count",
  "all_reusable_owned_stats_match_owned",
  "mean_reusable_owned_visited_nodes",
  "mean_reusable_owned_retained_leaves",
  "mean_reusable_owned_reconstructed_records",
  "mean_reusable_owned_matched_records",
  "mean_fse_visited_nodes",
  "mean_fse_retained_leaves",
  "mean_fse_reconstructed_records",
  "mean_fse_matched_records",
  "mean_baseline_evaluated_records",
  "mean_baseline_matched_records",
  "mean_candidate_ratio",
  "mean_reconstruction_avoidance_ratio",
  "mean_nodes_per_record"
) `
  -Rows $SummaryRows

$MissingRows = @($DetailObjects | Where-Object { $_.Status -ne "loaded" }).Count
$LoadedRows = @($DetailObjects | Where-Object { $_.Status -eq "loaded" }).Count
$ExpectedRows = $TrialWorkloadCsvPaths.Count * $TreeBaselines.Count

Set-Utf8Text -Path $NotesPath -Text "Target workload trial summary`r`n"
Add-Utf8Text -Path $NotesPath -Text "==============================`r`n`r`n"
Add-Utf8Text -Path $NotesPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $NotesPath -Text "Input label: $InputLabel`r`n"
Add-Utf8Text -Path $NotesPath -Text "Target workload: $TargetWorkloadName`r`n"
Add-Utf8Text -Path $NotesPath -Text "Input trial directory: $TrialInputDir`r`n"
Add-Utf8Text -Path $NotesPath -Text "Trial workload CSVs: $($TrialWorkloadCsvPaths.Count)`r`n"
Add-Utf8Text -Path $NotesPath -Text "Expected baseline rows: $ExpectedRows`r`n"
Add-Utf8Text -Path $NotesPath -Text "Loaded target rows: $LoadedRows`r`n"
Add-Utf8Text -Path $NotesPath -Text "Missing target rows: $MissingRows`r`n"
Add-Utf8Text -Path $NotesPath -Text "Details CSV: $DetailCsvPath`r`n"
Add-Utf8Text -Path $NotesPath -Text "Summary CSV: $SummaryCsvPath`r`n"

Add-Utf8Text -Path $NotesPath -Text "`r`nSummary rows`r`n"
Add-Utf8Text -Path $NotesPath -Text "------------`r`n"

foreach ($Summary in $SummaryObjects) {
  Add-Utf8Text -Path $NotesPath -Text "`r`n$($Summary.BaselineName)`r`n"
  Add-Utf8Text -Path $NotesPath -Text ("-" * $Summary.BaselineName.Length)
  Add-Utf8Text -Path $NotesPath -Text "`r`n"
  Add-Utf8Text -Path $NotesPath -Text "workload: $($Summary.WorkloadName)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "trials loaded: $($Summary.TrialCount)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean timing ratio: $(Format-InvariantDouble -Value $Summary.MeanTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "range timing ratio: $(Format-InvariantDouble -Value $Summary.RangeTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "sample stddev timing ratio: $(Format-InvariantDouble -Value $Summary.SampleStdDevTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean FSE average ns: $(Format-InvariantDouble -Value $Summary.MeanFseAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean baseline average ns: $(Format-InvariantDouble -Value $Summary.MeanBaselineAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean count-only average ns: $(Format-InvariantDouble -Value $Summary.MeanCountOnlyAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference average ns: $(Format-InvariantDouble -Value $Summary.MeanReferenceAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned average ns: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "all count-only stats match owned: $($Summary.AllCountOnlyStatsMatchOwned.ToString().ToLowerInvariant())`r`n"
  Add-Utf8Text -Path $NotesPath -Text "all reference stats match count-only: $($Summary.AllReferenceStatsMatchCountOnly.ToString().ToLowerInvariant())`r`n"
  Add-Utf8Text -Path $NotesPath -Text "all reusable owned stats match owned: $($Summary.AllReusableOwnedStatsMatchOwned.ToString().ToLowerInvariant())`r`n"
}

Add-Utf8Text -Path $NotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $NotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $NotesPath -Text "Use target workload summaries for focused boundary-workload review across repeated trials.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Do not accept timing movement if the target workload changed unexpectedly or any stats-equivalence field is false.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Use materialization summaries for dataset-wide output-contract movement and target summaries for the selected boundary workload.`r`n"

Write-Host ""
Write-Host "Target workload trial artifacts written:"
Write-Host "  target workload: $TargetWorkloadName"
Write-Host "  trial workload CSVs: $($TrialWorkloadCsvPaths.Count)"
Write-Host "  $DetailCsvPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $NotesPath"
