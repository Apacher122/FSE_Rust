param(
  [string]$InputWorkloadsCsv,
  [string]$OutputCsv,
  [string]$OutputNotesPath = ""
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

if ([string]::IsNullOrWhiteSpace($InputWorkloadsCsv)) {
  throw "`-InputWorkloadsCsv` cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($OutputCsv)) {
  throw "`-OutputCsv` cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($OutputNotesPath)) {
  $OutputNotesPath = [System.IO.Path]::ChangeExtension($OutputCsv, ".txt")
}

if (!(Test-Path -LiteralPath $InputWorkloadsCsv)) {
  throw "input workloads CSV was not found: $InputWorkloadsCsv"
}

$Rows = @(Import-Csv -LiteralPath $InputWorkloadsCsv)

if ($Rows.Count -eq 0) {
  throw "input workloads CSV has no rows: $InputWorkloadsCsv"
}

$RequiredFields = @(
  "dataset_records",
  "timing_iterations",
  "max_depth",
  "index_valid",
  "leaf_cardinality_valid",
  "baseline_name",
  "candidate_ratio",
  "fse_reconstructed_records",
  "fse_matched_records",
  "fse_average_elapsed_ns",
  "count_only_reconstructed_records",
  "count_only_matched_records",
  "count_only_average_elapsed_ns",
  "estimated_owned_result_overhead_ns",
  "count_only_speedup_ratio",
  "count_only_stats_match_owned"
)

foreach ($FieldName in $RequiredFields) {
  Require-CountOnlyCsvField `
    -Row $Rows[0] `
    -FieldName $FieldName `
    -CsvDescription "workload"
}

$Summaries = [ordered]@{}

foreach ($Row in $Rows) {
  $CandidateRatio = Convert-ToInvariantDoubleOrZero -Value $Row.candidate_ratio
  $SelectivityBucket = Get-CountOnlySelectivityBucket -CandidateRatio $CandidateRatio
  $Key = New-CountOnlySummaryKey `
    -BaselineName $Row.baseline_name `
    -SelectivityBucket $SelectivityBucket

  if (!$Summaries.Contains($Key)) {
    $Summaries[$Key] = New-CountOnlySummaryRow -SourceRow $Row -SelectivityBucket $SelectivityBucket
  }

  $Summary = $Summaries[$Key]

  $Summary.WorkloadCount += 1

  if ($Row.count_only_stats_match_owned -eq "true") {
    $Summary.StatsAgreeCount += 1
  }

  $Summary.TotalFseReconstructedRecords += Convert-ToInvariantIntOrZero -Value $Row.fse_reconstructed_records
  $Summary.TotalCountOnlyReconstructedRecords += Convert-ToInvariantIntOrZero -Value $Row.count_only_reconstructed_records
  $Summary.TotalFseMatchedRecords += Convert-ToInvariantIntOrZero -Value $Row.fse_matched_records
  $Summary.TotalCountOnlyMatchedRecords += Convert-ToInvariantIntOrZero -Value $Row.count_only_matched_records
  $Summary.CandidateRatioSum += $CandidateRatio
  $Summary.OwnedAverageElapsedNsSum += Convert-ToInvariantDoubleOrZero -Value $Row.fse_average_elapsed_ns
  $Summary.CountOnlyAverageElapsedNsSum += Convert-ToInvariantDoubleOrZero -Value $Row.count_only_average_elapsed_ns
  $Summary.OwnedResultOverheadNsSum += Convert-ToInvariantDoubleOrZero -Value $Row.estimated_owned_result_overhead_ns
  $Summary.CountOnlySpeedupRatioSum += Convert-ToInvariantDoubleOrZero -Value $Row.count_only_speedup_ratio
}

$Header = @(
  "dataset_records",
  "timing_iterations",
  "max_depth",
  "index_valid",
  "leaf_cardinality_valid",
  "baseline_name",
  "selectivity_bucket",
  "workload_count",
  "stats_agree_count",
  "all_stats_match_owned",
  "total_fse_reconstructed_records",
  "total_count_only_reconstructed_records",
  "total_fse_matched_records",
  "total_count_only_matched_records",
  "mean_candidate_ratio",
  "mean_owned_average_elapsed_ns",
  "mean_count_only_average_elapsed_ns",
  "mean_owned_result_overhead_ns",
  "mean_count_only_speedup_ratio",
  "weighted_count_only_speedup_ratio",
  "total_owned_average_elapsed_ns",
  "total_count_only_average_elapsed_ns",
  "total_owned_result_overhead_ns"
)

$OutputRows = New-Object System.Collections.Generic.List[string]
$OutputRows.Add((Convert-ToCsvRow -Values $Header))

$CalculatedSummaries = foreach ($Summary in $Summaries.Values) {
  New-CountOnlyCalculatedSummary -Summary $Summary
}

foreach ($Summary in $CalculatedSummaries) {
  $OutputRows.Add((Convert-ToCsvRow -Values @(
        $Summary.DatasetRecords,
        $Summary.TimingIterations,
        $Summary.MaxDepth,
        $Summary.IndexValid,
        $Summary.LeafCardinalityValid,
        $Summary.BaselineName,
        $Summary.SelectivityBucket,
        $Summary.WorkloadCount,
        $Summary.StatsAgreeCount,
        $Summary.AllStatsMatchOwned.ToString().ToLowerInvariant(),
        $Summary.TotalFseReconstructedRecords,
        $Summary.TotalCountOnlyReconstructedRecords,
        $Summary.TotalFseMatchedRecords,
        $Summary.TotalCountOnlyMatchedRecords,
        (Format-InvariantDouble -Value $Summary.MeanCandidateRatio),
        (Format-InvariantDouble -Value $Summary.MeanOwnedAverageElapsedNs),
        (Format-InvariantDouble -Value $Summary.MeanCountOnlyAverageElapsedNs),
        (Format-InvariantDouble -Value $Summary.MeanOwnedResultOverheadNs),
        (Format-InvariantDouble -Value $Summary.MeanCountOnlySpeedupRatio),
        (Format-InvariantDouble -Value $Summary.WeightedCountOnlySpeedupRatio),
        (Format-InvariantDouble -Value $Summary.TotalOwnedAverageElapsedNs),
        (Format-InvariantDouble -Value $Summary.TotalCountOnlyAverageElapsedNs),
        (Format-InvariantDouble -Value $Summary.TotalOwnedResultOverheadNs)
      )))
}

$OutputDirectory = Split-Path -Parent $OutputCsv

if (![string]::IsNullOrWhiteSpace($OutputDirectory)) {
  New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
}

$OutputText = (($OutputRows.ToArray()) -join "`r`n") + "`r`n"
Set-Utf8Text -Path $OutputCsv -Text $OutputText

$NotesDirectory = Split-Path -Parent $OutputNotesPath

if (![string]::IsNullOrWhiteSpace($NotesDirectory)) {
  New-Item -ItemType Directory -Force -Path $NotesDirectory | Out-Null
}

$NotesRows = New-Object System.Collections.Generic.List[string]

function Add-NotesLine {
  param(
    [string]$Text = ""
  )

  $NotesRows.Add($Text)
}

$FirstSummary = $CalculatedSummaries | Select-Object -First 1
$RowsWithStatsMismatch = @($CalculatedSummaries | Where-Object { !$_.AllStatsMatchOwned })
$LowBucketRows = @($CalculatedSummaries | Where-Object { $_.SelectivityBucket -eq "low" })
$FullBucketRows = @($CalculatedSummaries | Where-Object { $_.SelectivityBucket -eq "full" })
$BestLowBucketRow = $LowBucketRows |
Sort-Object -Property WeightedCountOnlySpeedupRatio -Descending |
Select-Object -First 1

Add-NotesLine "Count-only workload summary"
Add-NotesLine "==========================="
Add-NotesLine ""
Add-NotesLine "Input workloads CSV: $InputWorkloadsCsv"
Add-NotesLine "Output CSV: $OutputCsv"
Add-NotesLine "Dataset records: $($FirstSummary.DatasetRecords)"
Add-NotesLine "Timing iterations: $($FirstSummary.TimingIterations)"
Add-NotesLine "Max depth: $($FirstSummary.MaxDepth)"
Add-NotesLine "Index valid: $($FirstSummary.IndexValid)"
Add-NotesLine "Leaf cardinality valid: $($FirstSummary.LeafCardinalityValid)"
Add-NotesLine ""

Add-NotesLine "Stats agreement"
Add-NotesLine "---------------"

if ($RowsWithStatsMismatch.Count -eq 0) {
  Add-NotesLine "all count-only summary rows match owned-result structural stats: true"
}
else {
  Add-NotesLine "all count-only summary rows match owned-result structural stats: false"
  Add-NotesLine "mismatching summary rows: $($RowsWithStatsMismatch.Count)"
}

Add-NotesLine ""

Add-NotesLine "Low-selectivity count-only speedup"
Add-NotesLine "----------------------------------"

if ($LowBucketRows.Count -eq 0) {
  Add-NotesLine "no low-selectivity rows found"
}
else {
  Add-NotesLine "baseline | workloads | owned avg ns | count-only avg ns | owned overhead ns | weighted speedup"

  foreach ($Row in ($LowBucketRows | Sort-Object -Property BaselineName)) {
    Add-NotesLine "$($Row.BaselineName) | $($Row.WorkloadCount) | $(Format-InvariantDouble -Value $Row.MeanOwnedAverageElapsedNs) | $(Format-InvariantDouble -Value $Row.MeanCountOnlyAverageElapsedNs) | $(Format-InvariantDouble -Value $Row.MeanOwnedResultOverheadNs) | $(Format-InvariantDouble -Value $Row.WeightedCountOnlySpeedupRatio)x"
  }

  Add-NotesLine ""

  Add-NotesLine "best low-selectivity weighted speedup:"
  Add-NotesLine "baseline: $($BestLowBucketRow.BaselineName)"
  Add-NotesLine "weighted speedup: $(Format-InvariantDouble -Value $BestLowBucketRow.WeightedCountOnlySpeedupRatio)x"
  Add-NotesLine "owned avg ns: $(Format-InvariantDouble -Value $BestLowBucketRow.MeanOwnedAverageElapsedNs)"
  Add-NotesLine "count-only avg ns: $(Format-InvariantDouble -Value $BestLowBucketRow.MeanCountOnlyAverageElapsedNs)"
}

Add-NotesLine ""

Add-NotesLine "Full-selectivity behavior"
Add-NotesLine "-------------------------"

if ($FullBucketRows.Count -eq 0) {
  Add-NotesLine "no full-selectivity rows found"
}
else {
  Add-NotesLine "full-selectivity count-only speedups can be very large because count-only root coverage can return cardinality without materializing all owned Vector results"
  Add-NotesLine ""
  Add-NotesLine "baseline | workloads | owned avg ns | count-only avg ns | weighted speedup"

  foreach ($Row in ($FullBucketRows | Sort-Object -Property BaselineName)) {
    Add-NotesLine "$($Row.BaselineName) | $($Row.WorkloadCount) | $(Format-InvariantDouble -Value $Row.MeanOwnedAverageElapsedNs) | $(Format-InvariantDouble -Value $Row.MeanCountOnlyAverageElapsedNs) | $(Format-InvariantDouble -Value $Row.WeightedCountOnlySpeedupRatio)x"
  }
}

Add-NotesLine ""

Add-NotesLine "Interpretation"
Add-NotesLine "--------------"
Add-NotesLine "Use owned-result benchmark timing when the caller needs materialized rows."
Add-NotesLine "Use count-only benchmark timing when the caller only needs exact cardinality."
Add-NotesLine "Do not compare count-only timing and owned-result timing as if they answer the same application question."
Add-NotesLine "The count-only summary is valid only when all_stats_match_owned is true."

$NotesText = (($NotesRows.ToArray()) -join "`r`n") + "`r`n"
Set-Utf8Text -Path $OutputNotesPath -Text $NotesText

Write-Host "Count-only workload summary written: $OutputCsv"
Write-Host "Count-only workload notes written: $OutputNotesPath"
