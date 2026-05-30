param(
  [string]$Label = "local",
  [string]$OutputDir = "benchmark_artifacts",
  [int]$ExpectedTrials = 0,
  [switch]$RequireComparisons,
  [switch]$AllowSkippedTargetWorkloadReview
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$BenchmarkReviewManifestLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-review-manifest.ps1"

if (!(Test-Path -LiteralPath $BenchmarkReviewManifestLibrary)) {
  throw "benchmark review manifest helper library was not found: $BenchmarkReviewManifestLibrary"
}

. $BenchmarkReviewManifestLibrary

if ([string]::IsNullOrWhiteSpace($Label)) {
  throw "-Label cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
  throw "-OutputDir cannot be empty"
}

if ($ExpectedTrials -lt 0) {
  throw "-ExpectedTrials cannot be negative"
}

if ($Label.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0) {
  throw "-Label contains characters that cannot be used in artifact file names: $Label"
}

$Failures = @()

function Add-Failure {
  param(
    [string]$Message
  )

  $script:Failures += $Message
}

$AddValidationFailure = {
  param($Message)

  Add-Failure $Message
}


function Test-BenchmarkReviewTextColumnAllEquals {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string]$ExpectedValue,
    [string]$Description
  )

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue) {
      Add-Failure "$Description has missing value in column '$ColumnName'"
      continue
    }

    $TextValue = $RawValue.ToString().Trim().ToLowerInvariant()
    $ExpectedText = $ExpectedValue.Trim().ToLowerInvariant()

    if ($TextValue -ne $ExpectedText) {
      Add-Failure "$Description has unexpected value in column '$ColumnName'. expected '$ExpectedText', found '$TextValue'"
    }
  }
}


function Fail-Validation {
  param(
    [string]$ManifestPath,
    [int]$ManifestEntryCount
  )

  Write-BenchmarkReviewValidationReport `
    -Label $Label `
    -OutputDir $OutputDir `
    -ManifestPath $ManifestPath `
    -ExpectedTrials $ExpectedTrials `
    -RequireComparisons ([bool]$RequireComparisons) `
    -AllowSkippedTargetWorkloadReview ([bool]$AllowSkippedTargetWorkloadReview) `
    -ManifestEntryCount $ManifestEntryCount `
    -Failures $Failures

  throw "benchmark review artifact validation failed"
}




$ManifestLoadResult = Get-BenchmarkReviewManifestLoadResult `
  -OutputDir $OutputDir `
  -Label $Label `
  -RequireComparisons ([bool]$RequireComparisons) `
  -AllowSkippedTargetWorkloadReview ([bool]$AllowSkippedTargetWorkloadReview) `
  -OnFailure $AddValidationFailure `
  -OnInvalidPath {
  param($InvalidPath)

  Add-Failure "invalid path: $InvalidPath"
}

$ManifestPath = $ManifestLoadResult.ManifestPath
$ManifestEntries = @($ManifestLoadResult.Entries)

if ($ManifestLoadResult.HasBlockingFailures -or $Failures.Count -gt 0) {
  Fail-Validation -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
}

$ResolvedArtifacts = $ManifestLoadResult.ResolvedArtifacts

$BenchmarkOutputEntry = $ResolvedArtifacts.BenchmarkOutput
$DebugOutputEntry = $ResolvedArtifacts.DebugOutput
$SummaryEntry = $ResolvedArtifacts.Summary
$WorkloadsEntry = $ResolvedArtifacts.Workloads
$CountOnlySummaryEntry = $ResolvedArtifacts.CountOnlySummary
$CountOnlyNotesEntry = $ResolvedArtifacts.CountOnlyNotes
$MaterializationSummaryEntry = $ResolvedArtifacts.MaterializationSummary
$MaterializationNotesEntry = $ResolvedArtifacts.MaterializationNotes
$LowGapEntry = $ResolvedArtifacts.LowGap
$RegressionNotesEntry = $ResolvedArtifacts.RegressionNotes
$TrialSummaryEntry = $ResolvedArtifacts.TrialSummary
$TrialNotesEntry = $ResolvedArtifacts.TrialNotes
$WorkloadSummaryEntry = $ResolvedArtifacts.WorkloadSummary
$TargetSummaryEntry = $ResolvedArtifacts.TargetSummary
$TargetNotesEntry = $ResolvedArtifacts.TargetNotes
$ReviewNotesEntry = $ResolvedArtifacts.ReviewNotes
$ReviewManifestEntry = $ResolvedArtifacts.ReviewManifest
$AggregateComparisonEntry = $ResolvedArtifacts.AggregateComparison
$WorkloadComparisonEntry = $ResolvedArtifacts.WorkloadComparison
$CountOnlyComparisonEntry = $ResolvedArtifacts.CountOnlyComparison
$MaterializationComparisonEntry = $ResolvedArtifacts.MaterializationComparison
$TargetComparisonEntry = $ResolvedArtifacts.TargetComparison

Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $BenchmarkOutputEntry -Description "benchmark output"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $DebugOutputEntry -Description "debug output"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $CountOnlyNotesEntry -Description "count-only workload notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $MaterializationNotesEntry -Description "materialization mode notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $RegressionNotesEntry -Description "low-gap regression notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TrialNotesEntry -Description "low-gap trial notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TargetNotesEntry -Description "target workload trial notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $ReviewNotesEntry -Description "low-gap review notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $ReviewManifestEntry -Description "low-gap review manifest"

Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
  -Entry $SummaryEntry `
  -Description "summary CSV" `
  -RequiredColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "hierarchy_topology_valid",
  "parent_child_bounds_valid",
  "baseline_name"
) `
  -MinimumRows 3 `
  -BooleanTrueColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "hierarchy_topology_valid",
  "parent_child_bounds_valid"
) `
  -RequireBaselines $true

Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
  -Entry $WorkloadsEntry `
  -Description "workloads CSV" `
  -RequiredColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "baseline_name",
  "workload_name",
  "count_only_stats_match_owned"
) `
  -MinimumRows 1 `
  -BooleanTrueColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "count_only_stats_match_owned"
) `
  -RequireBaselines $true

Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
  -Entry $CountOnlySummaryEntry `
  -Description "count-only workload summary" `
  -RequiredColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "baseline_name",
  "selectivity_bucket",
  "all_stats_match_owned"
) `
  -MinimumRows 1 `
  -BooleanTrueColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "all_stats_match_owned"
) `
  -RequireBaselines $true

$MaterializationSummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $MaterializationSummaryEntry `
  -Description "materialization mode summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "fresh_owned_elapsed",
  "reusable_owned_elapsed",
  "reference_elapsed",
  "count_only_elapsed",
  "agreement"
) `
  -MinimumRows 1 `
  -OnFailure $AddValidationFailure

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $MaterializationSummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "materialization mode summary"

Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
  -Entry $LowGapEntry `
  -Description "low-selectivity gap CSV" `
  -RequiredColumns @(
  "baseline_name",
  "low_workload_count",
  "low_weighted_candidate_ratio",
  "low_mean_timing_ratio"
) `
  -MinimumRows 2 `
  -RequireBaselines $true

Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
  -Entry $TrialSummaryEntry `
  -Description "low-gap trial summary" `
  -RequiredColumns @(
  "baseline_name",
  "trial_count",
  "mean_low_mean_timing_ratio",
  "classification"
) `
  -MinimumRows 2 `
  -RequireBaselines $true `
  -TrialCountColumn "trial_count" `
  -ExpectedTrials $ExpectedTrials

Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
  -Entry $WorkloadSummaryEntry `
  -Description "low-gap workload trial summary" `
  -RequiredColumns @(
  "baseline_name",
  "trial_count",
  "most_frequent_weakest_workload",
  "mean_weakest_timing_ratio"
) `
  -MinimumRows 2 `
  -RequireBaselines $true `
  -TrialCountColumn "trial_count" `
  -ExpectedTrials $ExpectedTrials

if ($null -ne $TargetSummaryEntry -and $TargetSummaryEntry.State -eq $BenchmarkReviewManifestStateFound) {
  Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
    -Entry $TargetSummaryEntry `
    -Description "target workload trial summary" `
    -RequiredColumns @(
    "baseline_name",
    "workload_name",
    "trial_count",
    "mean_timing_ratio",
    "mean_candidate_ratio",
    "all_count_only_stats_match_owned",
    "all_reference_stats_match_count_only",
    "all_reusable_owned_stats_match_owned"
  ) `
    -MinimumRows 2 `
    -BooleanTrueColumns @(
    "all_count_only_stats_match_owned",
    "all_reference_stats_match_count_only",
    "all_reusable_owned_stats_match_owned"
  ) `
    -RequireBaselines $true `
    -TrialCountColumn "trial_count" `
    -ExpectedTrials $ExpectedTrials
}


if ($null -ne $TargetComparisonEntry -and $TargetComparisonEntry.State -eq $BenchmarkReviewManifestStateFound) {
  Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
    -Entry $TargetComparisonEntry `
    -Description "target workload trial comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "previous_workload_name",
    "current_workload_name",
    "previous_mean_timing_ratio",
    "current_mean_timing_ratio",
    "timing_classification",
    "previous_all_count_only_stats_match_owned",
    "current_all_count_only_stats_match_owned",
    "previous_all_reference_stats_match_count_only",
    "current_all_reference_stats_match_count_only",
    "previous_all_reusable_owned_stats_match_owned",
    "current_all_reusable_owned_stats_match_owned"
  ) `
    -MinimumRows 2 `
    -BooleanTrueColumns @(
    "previous_all_count_only_stats_match_owned",
    "current_all_count_only_stats_match_owned",
    "previous_all_reference_stats_match_count_only",
    "current_all_reference_stats_match_count_only",
    "previous_all_reusable_owned_stats_match_owned",
    "current_all_reusable_owned_stats_match_owned"
  ) `
    -RequireBaselines $true
}


if ($null -ne $MaterializationComparisonEntry -and $MaterializationComparisonEntry.State -eq $BenchmarkReviewManifestStateFound) {
  $MaterializationComparisonRows = Read-BenchmarkReviewPolicyValidatedCsv `
    -Entry $MaterializationComparisonEntry `
    -Description "materialization mode comparison CSV" `
    -RequiredColumns @(
    "dataset",
    "max_depth",
    "workload_name",
    "matched_records_delta",
    "fresh_owned_elapsed_classification",
    "reusable_owned_elapsed_classification",
    "reference_elapsed_classification",
    "count_only_elapsed_classification",
    "agreement_status"
  ) `
    -MinimumRows 1 `
    -OnFailure $AddValidationFailure

  Test-BenchmarkReviewTextColumnAllEquals `
    -Rows $MaterializationComparisonRows `
    -ColumnName "agreement_status" `
    -ExpectedValue "pass" `
    -Description "materialization mode comparison CSV"
}

if ($RequireComparisons) {
  Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
    -Entry $AggregateComparisonEntry `
    -Description "aggregate trial comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "previous_mean_low_mean_timing_ratio",
    "current_mean_low_mean_timing_ratio",
    "timing_classification"
  ) `
    -MinimumRows 2 `
    -RequireBaselines $true

  Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
    -Entry $WorkloadComparisonEntry `
    -Description "workload trial comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "previous_mean_weakest_timing_ratio",
    "current_mean_weakest_timing_ratio",
    "timing_classification"
  ) `
    -MinimumRows 2 `
    -RequireBaselines $true

  Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
    -Entry $CountOnlyComparisonEntry `
    -Description "count-only workload comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "selectivity_bucket",
    "previous_weighted_count_only_speedup_ratio",
    "current_weighted_count_only_speedup_ratio",
    "weighted_count_only_speedup_classification"
  ) `
    -MinimumRows 1 `
    -RequireBaselines $true

}

if ($Failures.Count -gt 0) {
  Fail-Validation -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
}

Write-BenchmarkReviewValidationReport `
  -Label $Label `
  -OutputDir $OutputDir `
  -ManifestPath $ManifestPath `
  -ExpectedTrials $ExpectedTrials `
  -RequireComparisons ([bool]$RequireComparisons) `
  -AllowSkippedTargetWorkloadReview ([bool]$AllowSkippedTargetWorkloadReview) `
  -ManifestEntryCount $ManifestEntries.Count `
  -Failures $Failures

Write-Host "Benchmark review artifacts are valid."