param(
  [string]$Label = "local",
  [string]$PreviousCountOnlySummaryCsv,
  [string]$CurrentCountOnlySummaryCsv,
  [string]$OutputDir = "benchmark_artifacts",
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"
$BenchmarkCountOnlyLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-count-only.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkCountOnlyLibrary)) {
  throw "benchmark count-only helper library was not found: $BenchmarkCountOnlyLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkCountOnlyLibrary

if ([string]::IsNullOrWhiteSpace($PreviousCountOnlySummaryCsv)) {
  throw "`-PreviousCountOnlySummaryCsv` is required"
}

if ([string]::IsNullOrWhiteSpace($CurrentCountOnlySummaryCsv)) {
  throw "`-CurrentCountOnlySummaryCsv` is required"
}

if (!(Test-Path -LiteralPath $PreviousCountOnlySummaryCsv)) {
  throw "previous count-only summary CSV was not found: $PreviousCountOnlySummaryCsv"
}

if (!(Test-Path -LiteralPath $CurrentCountOnlySummaryCsv)) {
  throw "current count-only summary CSV was not found: $CurrentCountOnlySummaryCsv"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$ComparisonCsvPath = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.csv"
$ComparisonNotesPath = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.txt"


function New-ComparisonRow {
  param(
    [object]$PreviousRow,
    [object]$CurrentRow
  )

  $PreviousSpeedup = Convert-ToInvariantDouble -Value $PreviousRow.weighted_count_only_speedup_ratio
  $CurrentSpeedup = Convert-ToInvariantDouble -Value $CurrentRow.weighted_count_only_speedup_ratio
  $SpeedupDelta = Get-Delta -CurrentValue $CurrentSpeedup -PreviousValue $PreviousSpeedup

  $PreviousCountElapsed = Convert-ToInvariantDouble -Value $PreviousRow.mean_count_only_average_elapsed_ns
  $CurrentCountElapsed = Convert-ToInvariantDouble -Value $CurrentRow.mean_count_only_average_elapsed_ns
  $CountElapsedDelta = Get-Delta -CurrentValue $CurrentCountElapsed -PreviousValue $PreviousCountElapsed

  $PreviousOwnedElapsed = Convert-ToInvariantDouble -Value $PreviousRow.mean_owned_average_elapsed_ns
  $CurrentOwnedElapsed = Convert-ToInvariantDouble -Value $CurrentRow.mean_owned_average_elapsed_ns
  $OwnedElapsedDelta = Get-Delta -CurrentValue $CurrentOwnedElapsed -PreviousValue $PreviousOwnedElapsed

  $PreviousOwnedOverhead = Convert-ToInvariantDouble -Value $PreviousRow.mean_owned_result_overhead_ns
  $CurrentOwnedOverhead = Convert-ToInvariantDouble -Value $CurrentRow.mean_owned_result_overhead_ns
  $OwnedOverheadDelta = Get-Delta -CurrentValue $CurrentOwnedOverhead -PreviousValue $PreviousOwnedOverhead

  return [PSCustomObject]@{
    BaselineName                       = $CurrentRow.baseline_name
    SelectivityBucket                  = $CurrentRow.selectivity_bucket
    PreviousWorkloadCount              = $PreviousRow.workload_count
    CurrentWorkloadCount               = $CurrentRow.workload_count
    PreviousAllStatsMatchOwned         = $PreviousRow.all_stats_match_owned
    CurrentAllStatsMatchOwned          = $CurrentRow.all_stats_match_owned
    PreviousWeightedSpeedup            = $PreviousSpeedup
    CurrentWeightedSpeedup             = $CurrentSpeedup
    WeightedSpeedupDelta               = $SpeedupDelta
    WeightedSpeedupClassification      = Get-CountOnlyHigherIsBetterClassification `
      -Delta $SpeedupDelta `
      -NoiseThreshold $NoiseThreshold
    PreviousMeanCountOnlyElapsedNs     = $PreviousCountElapsed
    CurrentMeanCountOnlyElapsedNs      = $CurrentCountElapsed
    MeanCountOnlyElapsedDeltaNs        = $CountElapsedDelta
    MeanCountOnlyElapsedClassification = Get-CountOnlyLowerIsBetterClassification `
      -Delta $CountElapsedDelta `
      -ReferenceValue $PreviousCountElapsed `
      -NoiseThreshold $NoiseThreshold
    PreviousMeanOwnedElapsedNs         = $PreviousOwnedElapsed
    CurrentMeanOwnedElapsedNs          = $CurrentOwnedElapsed
    MeanOwnedElapsedDeltaNs            = $OwnedElapsedDelta
    PreviousMeanOwnedResultOverheadNs  = $PreviousOwnedOverhead
    CurrentMeanOwnedResultOverheadNs   = $CurrentOwnedOverhead
    MeanOwnedResultOverheadDeltaNs     = $OwnedOverheadDelta
  }
}

$PreviousRows = @(Import-Csv -LiteralPath $PreviousCountOnlySummaryCsv)
$CurrentRows = @(Import-Csv -LiteralPath $CurrentCountOnlySummaryCsv)

if ($PreviousRows.Count -eq 0) {
  throw "previous count-only summary CSV has no rows: $PreviousCountOnlySummaryCsv"
}

if ($CurrentRows.Count -eq 0) {
  throw "current count-only summary CSV has no rows: $CurrentCountOnlySummaryCsv"
}

$RequiredFields = @(
  "baseline_name",
  "selectivity_bucket",
  "workload_count",
  "all_stats_match_owned",
  "mean_owned_average_elapsed_ns",
  "mean_count_only_average_elapsed_ns",
  "mean_owned_result_overhead_ns",
  "weighted_count_only_speedup_ratio"
)

foreach ($FieldName in $RequiredFields) {
  Require-CountOnlyCsvField `
    -Row $PreviousRows[0] `
    -FieldName $FieldName `
    -CsvDescription "count-only summary"
  Require-CountOnlyCsvField `
    -Row $CurrentRows[0] `
    -FieldName $FieldName `
    -CsvDescription "count-only summary"
}

$PreviousMap = New-CountOnlySummaryRowMap -Rows $PreviousRows
$CurrentMap = New-CountOnlySummaryRowMap -Rows $CurrentRows

$ComparisonRows = New-Object System.Collections.Generic.List[object]
$MissingPreviousKeys = New-Object System.Collections.Generic.List[string]
$MissingCurrentKeys = New-Object System.Collections.Generic.List[string]

foreach ($Key in $CurrentMap.Keys) {
  if (!$PreviousMap.ContainsKey($Key)) {
    $MissingPreviousKeys.Add($Key)
    continue
  }

  $ComparisonRows.Add((New-ComparisonRow `
        -PreviousRow $PreviousMap[$Key] `
        -CurrentRow $CurrentMap[$Key]))
}

foreach ($Key in $PreviousMap.Keys) {
  if (!$CurrentMap.ContainsKey($Key)) {
    $MissingCurrentKeys.Add($Key)
  }
}

$Header = @(
  "baseline_name",
  "selectivity_bucket",
  "previous_workload_count",
  "current_workload_count",
  "previous_all_stats_match_owned",
  "current_all_stats_match_owned",
  "previous_weighted_count_only_speedup_ratio",
  "current_weighted_count_only_speedup_ratio",
  "weighted_count_only_speedup_delta",
  "weighted_count_only_speedup_classification",
  "previous_mean_count_only_average_elapsed_ns",
  "current_mean_count_only_average_elapsed_ns",
  "mean_count_only_average_elapsed_delta_ns",
  "mean_count_only_average_elapsed_classification",
  "previous_mean_owned_average_elapsed_ns",
  "current_mean_owned_average_elapsed_ns",
  "mean_owned_average_elapsed_delta_ns",
  "previous_mean_owned_result_overhead_ns",
  "current_mean_owned_result_overhead_ns",
  "mean_owned_result_overhead_delta_ns"
)

$CsvRows = New-Object System.Collections.Generic.List[object]

foreach ($Row in $ComparisonRows) {
  $CsvRows.Add(@(
      $Row.BaselineName,
      $Row.SelectivityBucket,
      $Row.PreviousWorkloadCount,
      $Row.CurrentWorkloadCount,
      $Row.PreviousAllStatsMatchOwned,
      $Row.CurrentAllStatsMatchOwned,
      (Format-InvariantDouble -Value $Row.PreviousWeightedSpeedup),
      (Format-InvariantDouble -Value $Row.CurrentWeightedSpeedup),
      (Format-InvariantDouble -Value $Row.WeightedSpeedupDelta),
      $Row.WeightedSpeedupClassification,
      (Format-InvariantDouble -Value $Row.PreviousMeanCountOnlyElapsedNs),
      (Format-InvariantDouble -Value $Row.CurrentMeanCountOnlyElapsedNs),
      (Format-InvariantDouble -Value $Row.MeanCountOnlyElapsedDeltaNs),
      $Row.MeanCountOnlyElapsedClassification,
      (Format-InvariantDouble -Value $Row.PreviousMeanOwnedElapsedNs),
      (Format-InvariantDouble -Value $Row.CurrentMeanOwnedElapsedNs),
      (Format-InvariantDouble -Value $Row.MeanOwnedElapsedDeltaNs),
      (Format-InvariantDouble -Value $Row.PreviousMeanOwnedResultOverheadNs),
      (Format-InvariantDouble -Value $Row.CurrentMeanOwnedResultOverheadNs),
      (Format-InvariantDouble -Value $Row.MeanOwnedResultOverheadDeltaNs)
    ))
}

Write-CsvDocument -Path $ComparisonCsvPath -Header $Header -Rows $CsvRows

$StatsMismatchRows = @($ComparisonRows | Where-Object {
    $_.PreviousAllStatsMatchOwned -ne "true" -or $_.CurrentAllStatsMatchOwned -ne "true"
  })

$LowRows = @($ComparisonRows | Where-Object { $_.SelectivityBucket -eq "low" })
$ImprovedRows = @($LowRows | Where-Object { $_.WeightedSpeedupClassification -eq "improved" })
$RegressedRows = @($LowRows | Where-Object { $_.WeightedSpeedupClassification -eq "regressed" })
$StableRows = @($LowRows | Where-Object { $_.WeightedSpeedupClassification -eq "stable/noise" })

Set-Utf8Text -Path $ComparisonNotesPath -Text "Count-only workload summary comparison`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "======================================`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous summary CSV: $PreviousCountOnlySummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current summary CSV: $CurrentCountOnlySummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"

Add-Utf8Text -Path $ComparisonNotesPath -Text "Row coverage`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "comparison rows: $($ComparisonRows.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "missing previous rows: $($MissingPreviousKeys.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "missing current rows: $($MissingCurrentKeys.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"

Add-Utf8Text -Path $ComparisonNotesPath -Text "Stats agreement`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "---------------`r`n"

if ($StatsMismatchRows.Count -eq 0) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "all compared rows match owned-result structural stats in both summaries: true`r`n"
}
else {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "all compared rows match owned-result structural stats in both summaries: false`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mismatching rows: $($StatsMismatchRows.Count)`r`n"
}

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Low-selectivity speedup movement`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "--------------------------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "improved: $($ImprovedRows.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "stable/noise: $($StableRows.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "regressed: $($RegressedRows.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"

if ($LowRows.Count -eq 0) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "no low-selectivity rows found`r`n"
}
else {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "baseline | previous speedup | current speedup | delta | classification | count-only elapsed delta ns`r`n"

  foreach ($Row in ($LowRows | Sort-Object -Property BaselineName)) {
    Add-Utf8Text -Path $ComparisonNotesPath -Text "$($Row.BaselineName) | $(Format-InvariantDouble -Value $Row.PreviousWeightedSpeedup)x | $(Format-InvariantDouble -Value $Row.CurrentWeightedSpeedup)x | $(Format-InvariantDouble -Value $Row.WeightedSpeedupDelta) | $($Row.WeightedSpeedupClassification) | $(Format-InvariantDouble -Value $Row.MeanCountOnlyElapsedDeltaNs)`r`n"
  }
}

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Interpretation`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "--------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Use this comparison only for count-only query mode movement.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Do not treat count-only speedup movement as owned-result query movement.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "A higher weighted count-only speedup means count-only timing improved relative to owned-result timing for that bucket.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "A lower count-only elapsed value is better, but row-level timing is noisy at nanosecond scale.`r`n"

Write-Host "Count-only workload summary comparison written:"
Write-Host "  $ComparisonCsvPath"
Write-Host "  $ComparisonNotesPath"