param(
  [string]$Label = "local",
  [string]$PreviousWorkloadSummaryCsv,
  [string]$CurrentWorkloadSummaryCsv,
  [string]$OutputDir = "benchmark_artifacts",
  [double]$NoiseThreshold = 0.03
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

if ([string]::IsNullOrWhiteSpace($PreviousWorkloadSummaryCsv)) {
  throw "`-PreviousWorkloadSummaryCsv` is required"
}

if ([string]::IsNullOrWhiteSpace($CurrentWorkloadSummaryCsv)) {
  throw "`-CurrentWorkloadSummaryCsv` is required"
}

if (!(Test-Path -LiteralPath $PreviousWorkloadSummaryCsv)) {
  throw "previous workload summary CSV was not found: $PreviousWorkloadSummaryCsv"
}

if (!(Test-Path -LiteralPath $CurrentWorkloadSummaryCsv)) {
  throw "current workload summary CSV was not found: $CurrentWorkloadSummaryCsv"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$ComparisonCsvPath = Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.csv"
$ComparisonNotesPath = Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.txt"

$TreeBaselines = Get-BenchmarkTreeBaselineNames


function New-ComparisonRow {
  param(
    [string]$BaselineName,
    [object]$PreviousRow,
    [object]$CurrentRow
  )

  $PreviousWeakestWorkload = Get-TextField -Row $PreviousRow -FieldName "most_frequent_weakest_workload"
  $CurrentWeakestWorkload = Get-TextField -Row $CurrentRow -FieldName "most_frequent_weakest_workload"

  $PreviousMeanTiming = Get-Metric -Row $PreviousRow -FieldName "mean_weakest_timing_ratio"
  $CurrentMeanTiming = Get-Metric -Row $CurrentRow -FieldName "mean_weakest_timing_ratio"
  $TimingDelta = Get-Delta -CurrentValue $CurrentMeanTiming -PreviousValue $PreviousMeanTiming
  $TimingClassification = Get-Classification -Delta $TimingDelta -Threshold $NoiseThreshold

  $PreviousVisitedNodes = Get-Metric -Row $PreviousRow -FieldName "mean_fse_visited_nodes"
  $CurrentVisitedNodes = Get-Metric -Row $CurrentRow -FieldName "mean_fse_visited_nodes"
  $VisitedNodeDelta = Get-Delta -CurrentValue $CurrentVisitedNodes -PreviousValue $PreviousVisitedNodes

  $PreviousRetainedLeaves = Get-Metric -Row $PreviousRow -FieldName "mean_fse_retained_leaves"
  $CurrentRetainedLeaves = Get-Metric -Row $CurrentRow -FieldName "mean_fse_retained_leaves"
  $RetainedLeafDelta = Get-Delta -CurrentValue $CurrentRetainedLeaves -PreviousValue $PreviousRetainedLeaves

  $PreviousReconstructedRecords = Get-Metric -Row $PreviousRow -FieldName "mean_fse_reconstructed_records"
  $CurrentReconstructedRecords = Get-Metric -Row $CurrentRow -FieldName "mean_fse_reconstructed_records"
  $ReconstructedRecordDelta = Get-Delta -CurrentValue $CurrentReconstructedRecords -PreviousValue $PreviousReconstructedRecords

  $PreviousBaselineRecords = Get-Metric -Row $PreviousRow -FieldName "mean_baseline_evaluated_records"
  $CurrentBaselineRecords = Get-Metric -Row $CurrentRow -FieldName "mean_baseline_evaluated_records"
  $BaselineRecordDelta = Get-Delta -CurrentValue $CurrentBaselineRecords -PreviousValue $PreviousBaselineRecords

  $PreviousCandidateRatio = Get-Metric -Row $PreviousRow -FieldName "mean_candidate_ratio"
  $CurrentCandidateRatio = Get-Metric -Row $CurrentRow -FieldName "mean_candidate_ratio"
  $CandidateRatioDelta = Get-Delta -CurrentValue $CurrentCandidateRatio -PreviousValue $PreviousCandidateRatio

  $PreviousNodesPerRecord = Get-Metric -Row $PreviousRow -FieldName "mean_nodes_per_record"
  $CurrentNodesPerRecord = Get-Metric -Row $CurrentRow -FieldName "mean_nodes_per_record"
  $NodesPerRecordDelta = Get-Delta -CurrentValue $CurrentNodesPerRecord -PreviousValue $PreviousNodesPerRecord

  return [PSCustomObject]@{
    BaselineName                 = $BaselineName
    PreviousWeakestWorkload      = $PreviousWeakestWorkload
    CurrentWeakestWorkload       = $CurrentWeakestWorkload
    PreviousMeanTiming           = $PreviousMeanTiming
    CurrentMeanTiming            = $CurrentMeanTiming
    TimingDelta                  = $TimingDelta
    TimingClassification         = $TimingClassification
    PreviousVisitedNodes         = $PreviousVisitedNodes
    CurrentVisitedNodes          = $CurrentVisitedNodes
    VisitedNodeDelta             = $VisitedNodeDelta
    PreviousRetainedLeaves       = $PreviousRetainedLeaves
    CurrentRetainedLeaves        = $CurrentRetainedLeaves
    RetainedLeafDelta            = $RetainedLeafDelta
    PreviousReconstructedRecords = $PreviousReconstructedRecords
    CurrentReconstructedRecords  = $CurrentReconstructedRecords
    ReconstructedRecordDelta     = $ReconstructedRecordDelta
    PreviousBaselineRecords      = $PreviousBaselineRecords
    CurrentBaselineRecords       = $CurrentBaselineRecords
    BaselineRecordDelta          = $BaselineRecordDelta
    PreviousCandidateRatio       = $PreviousCandidateRatio
    CurrentCandidateRatio        = $CurrentCandidateRatio
    CandidateRatioDelta          = $CandidateRatioDelta
    PreviousNodesPerRecord       = $PreviousNodesPerRecord
    CurrentNodesPerRecord        = $CurrentNodesPerRecord
    NodesPerRecordDelta          = $NodesPerRecordDelta
  }
}

$PreviousResolvedPath = (Resolve-Path -LiteralPath $PreviousWorkloadSummaryCsv).Path
$CurrentResolvedPath = (Resolve-Path -LiteralPath $CurrentWorkloadSummaryCsv).Path

$PreviousRows = @(Import-Csv -LiteralPath $PreviousResolvedPath)
$CurrentRows = @(Import-Csv -LiteralPath $CurrentResolvedPath)

$ComparisonObjects = @(New-BenchmarkBaselineComparisonObjects `
    -PreviousRows $PreviousRows `
    -CurrentRows $CurrentRows `
    -BaselineNames $TreeBaselines `
    -RowFactory {
    param(
      [string]$BaselineName,
      [object]$PreviousRow,
      [object]$CurrentRow
    )

    New-ComparisonRow `
      -BaselineName $BaselineName `
      -PreviousRow $PreviousRow `
      -CurrentRow $CurrentRow
  })

$ComparisonRows = @()

foreach ($Comparison in $ComparisonObjects) {
  $ComparisonRows += , @(
    $Comparison.BaselineName,
    $Comparison.PreviousWeakestWorkload,
    $Comparison.CurrentWeakestWorkload,
    $(Format-InvariantDouble -Value $Comparison.PreviousMeanTiming),
    $(Format-InvariantDouble -Value $Comparison.CurrentMeanTiming),
    $(Format-InvariantDouble -Value $Comparison.TimingDelta),
    $Comparison.TimingClassification,
    $(Format-InvariantDouble -Value $Comparison.PreviousVisitedNodes),
    $(Format-InvariantDouble -Value $Comparison.CurrentVisitedNodes),
    $(Format-InvariantDouble -Value $Comparison.VisitedNodeDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousRetainedLeaves),
    $(Format-InvariantDouble -Value $Comparison.CurrentRetainedLeaves),
    $(Format-InvariantDouble -Value $Comparison.RetainedLeafDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousReconstructedRecords),
    $(Format-InvariantDouble -Value $Comparison.CurrentReconstructedRecords),
    $(Format-InvariantDouble -Value $Comparison.ReconstructedRecordDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousBaselineRecords),
    $(Format-InvariantDouble -Value $Comparison.CurrentBaselineRecords),
    $(Format-InvariantDouble -Value $Comparison.BaselineRecordDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousCandidateRatio),
    $(Format-InvariantDouble -Value $Comparison.CurrentCandidateRatio),
    $(Format-InvariantDouble -Value $Comparison.CandidateRatioDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousNodesPerRecord),
    $(Format-InvariantDouble -Value $Comparison.CurrentNodesPerRecord),
    $(Format-InvariantDouble -Value $Comparison.NodesPerRecordDelta)
  )
}

Write-CsvDocument `
  -Path $ComparisonCsvPath `
  -Header @(
  "baseline_name",
  "previous_weakest_workload",
  "current_weakest_workload",
  "previous_mean_weakest_timing_ratio",
  "current_mean_weakest_timing_ratio",
  "mean_weakest_timing_delta",
  "timing_classification",
  "previous_mean_fse_visited_nodes",
  "current_mean_fse_visited_nodes",
  "mean_fse_visited_nodes_delta",
  "previous_mean_fse_retained_leaves",
  "current_mean_fse_retained_leaves",
  "mean_fse_retained_leaves_delta",
  "previous_mean_fse_reconstructed_records",
  "current_mean_fse_reconstructed_records",
  "mean_fse_reconstructed_records_delta",
  "previous_mean_baseline_evaluated_records",
  "current_mean_baseline_evaluated_records",
  "mean_baseline_evaluated_records_delta",
  "previous_mean_candidate_ratio",
  "current_mean_candidate_ratio",
  "mean_candidate_ratio_delta",
  "previous_mean_nodes_per_record",
  "current_mean_nodes_per_record",
  "mean_nodes_per_record_delta"
) `
  -Rows $ComparisonRows

Set-Utf8Text -Path $ComparisonNotesPath -Text "Low-gap workload trial comparison`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "=================================`r`n`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous workload summary: $PreviousWorkloadSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous resolved path: $PreviousResolvedPath`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current workload summary: $CurrentWorkloadSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current resolved path: $CurrentResolvedPath`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Timing ratio meaning: baseline elapsed / FSE elapsed; higher is better for FSE.`r`n"

foreach ($Comparison in $ComparisonObjects) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n$($Comparison.BaselineName)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text ("-" * $Comparison.BaselineName.Length)
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous weakest workload: $($Comparison.PreviousWeakestWorkload)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current weakest workload: $($Comparison.CurrentWeakestWorkload)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous mean weakest timing: $(Format-InvariantDouble -Value $Comparison.PreviousMeanTiming)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current mean weakest timing: $(Format-InvariantDouble -Value $Comparison.CurrentMeanTiming)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean weakest timing delta: $(Format-InvariantDouble -Value $Comparison.TimingDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "classification: $($Comparison.TimingClassification)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE visited nodes delta: $(Format-InvariantDouble -Value $Comparison.VisitedNodeDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE retained leaves delta: $(Format-InvariantDouble -Value $Comparison.RetainedLeafDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean FSE reconstructed records delta: $(Format-InvariantDouble -Value $Comparison.ReconstructedRecordDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean baseline evaluated records delta: $(Format-InvariantDouble -Value $Comparison.BaselineRecordDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean candidate ratio delta: $(Format-InvariantDouble -Value $Comparison.CandidateRatioDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean nodes per record delta: $(Format-InvariantDouble -Value $Comparison.NodesPerRecordDelta)`r`n"
}

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "For boundary-focused query changes, prefer improvements beyond the noise threshold.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "A timing improvement is stronger when visited nodes, reconstructed records, and candidate ratio do not increase.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "If the weakest workload changes away from cluster_boundary_range, inspect the workload trial details before accepting the result.`r`n"

Write-Host ""
Write-Host "Low-gap workload comparison artifacts written:"
Write-Host "  $ComparisonCsvPath"
Write-Host "  $ComparisonNotesPath"