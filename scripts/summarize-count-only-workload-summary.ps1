param(
  [string]$InputWorkloadsCsv,
  [string]$OutputCsv,
  [string]$OutputNotesPath = ""
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

. $BenchmarkCsvLibrary

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

function New-CalculatedSummary {
  param(
    [object]$Summary
  )

  $WorkloadCount = [double]$Summary.WorkloadCount

  $MeanCandidateRatio = $Summary.CandidateRatioSum / $WorkloadCount
  $MeanOwnedAverageElapsedNs = $Summary.OwnedAverageElapsedNsSum / $WorkloadCount
  $MeanCountOnlyAverageElapsedNs = $Summary.CountOnlyAverageElapsedNsSum / $WorkloadCount
  $MeanOwnedResultOverheadNs = $Summary.OwnedResultOverheadNsSum / $WorkloadCount
  $MeanCountOnlySpeedupRatio = $Summary.CountOnlySpeedupRatioSum / $WorkloadCount

  if ($Summary.CountOnlyAverageElapsedNsSum -le 0.0) {
    $WeightedCountOnlySpeedupRatio = 0.0
  }
  else {
    $WeightedCountOnlySpeedupRatio =
    $Summary.OwnedAverageElapsedNsSum / $Summary.CountOnlyAverageElapsedNsSum
  }

  return [PSCustomObject]@{
    DatasetRecords                     = $Summary.DatasetRecords
    TimingIterations                   = $Summary.TimingIterations
    MaxDepth                           = $Summary.MaxDepth
    IndexValid                         = $Summary.IndexValid
    LeafCardinalityValid               = $Summary.LeafCardinalityValid
    BaselineName                       = $Summary.BaselineName
    SelectivityBucket                  = $Summary.SelectivityBucket
    WorkloadCount                      = $Summary.WorkloadCount
    StatsAgreeCount                    = $Summary.StatsAgreeCount
    AllStatsMatchOwned                 = $Summary.StatsAgreeCount -eq $Summary.WorkloadCount
    TotalFseReconstructedRecords       = $Summary.TotalFseReconstructedRecords
    TotalCountOnlyReconstructedRecords = $Summary.TotalCountOnlyReconstructedRecords
    TotalFseMatchedRecords             = $Summary.TotalFseMatchedRecords
    TotalCountOnlyMatchedRecords       = $Summary.TotalCountOnlyMatchedRecords
    MeanCandidateRatio                 = $MeanCandidateRatio
    MeanOwnedAverageElapsedNs          = $MeanOwnedAverageElapsedNs
    MeanCountOnlyAverageElapsedNs      = $MeanCountOnlyAverageElapsedNs
    MeanOwnedResultOverheadNs          = $MeanOwnedResultOverheadNs
    MeanCountOnlySpeedupRatio          = $MeanCountOnlySpeedupRatio
    WeightedCountOnlySpeedupRatio      = $WeightedCountOnlySpeedupRatio
    TotalOwnedAverageElapsedNs         = $Summary.OwnedAverageElapsedNsSum
    TotalCountOnlyAverageElapsedNs     = $Summary.CountOnlyAverageElapsedNsSum
    TotalOwnedResultOverheadNs         = $Summary.OwnedResultOverheadNsSum
  }
}

function Get-SelectivityBucket {
  param(
    [double]$CandidateRatio
  )

  if ($CandidateRatio -le 0.0) {
    return "empty"
  }

  if ($CandidateRatio -le 0.25) {
    return "low"
  }

  if ($CandidateRatio -le 0.50) {
    return "medium"
  }

  if ($CandidateRatio -lt 1.0) {
    return "high"
  }

  return "full"
}

function New-SummaryRow {
  param(
    [object]$SourceRow,
    [string]$SelectivityBucket
  )

  return [PSCustomObject]@{
    DatasetRecords                     = $SourceRow.dataset_records
    TimingIterations                   = $SourceRow.timing_iterations
    MaxDepth                           = $SourceRow.max_depth
    IndexValid                         = $SourceRow.index_valid
    LeafCardinalityValid               = $SourceRow.leaf_cardinality_valid
    BaselineName                       = $SourceRow.baseline_name
    SelectivityBucket                  = $SelectivityBucket
    WorkloadCount                      = 0
    StatsAgreeCount                    = 0
    TotalFseReconstructedRecords       = 0
    TotalCountOnlyReconstructedRecords = 0
    TotalFseMatchedRecords             = 0
    TotalCountOnlyMatchedRecords       = 0
    CandidateRatioSum                  = 0.0
    OwnedAverageElapsedNsSum           = 0.0
    CountOnlyAverageElapsedNsSum       = 0.0
    OwnedResultOverheadNsSum           = 0.0
    CountOnlySpeedupRatioSum           = 0.0
  }
}

function Require-Field {
  param(
    [object]$Row,
    [string]$FieldName
  )

  if ($null -eq $Row.PSObject.Properties[$FieldName]) {
    throw "required workload CSV field was not found: $FieldName"
  }
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
  Require-Field -Row $Rows[0] -FieldName $FieldName
}

$Summaries = [ordered]@{}

foreach ($Row in $Rows) {
  $CandidateRatio = Convert-ToInvariantDoubleOrZero -Value $Row.candidate_ratio
  $SelectivityBucket = Get-SelectivityBucket -CandidateRatio $CandidateRatio
  $Key = "$($Row.baseline_name)|$SelectivityBucket"

  if (!$Summaries.Contains($Key)) {
    $Summaries[$Key] = New-SummaryRow -SourceRow $Row -SelectivityBucket $SelectivityBucket
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
  New-CalculatedSummary -Summary $Summary
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