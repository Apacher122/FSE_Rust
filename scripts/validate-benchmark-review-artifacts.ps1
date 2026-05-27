param(
  [string]$Label = "local",
  [string]$OutputDir = "benchmark_artifacts",
  [int]$ExpectedTrials = 0,
  [switch]$RequireComparisons,
  [switch]$AllowSkippedTargetWorkloadReview
)

$ErrorActionPreference = "Stop"

$BenchmarkReviewManifestLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-review-manifest.ps1"

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

function Test-SafePathExists {
  param(
    [string]$Path
  )

  Test-BenchmarkReviewPathExists `
    -Path $Path `
    -OnInvalidPath {
    param($InvalidPath)

    Add-Failure "invalid path: $InvalidPath"
  }
}

function Resolve-ReviewArtifactPath {
  param(
    [string]$Path
  )

  Resolve-BenchmarkReviewArtifactPath `
    -Path $Path `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath {
    param($InvalidPath)

    Add-Failure "invalid path: $InvalidPath"
  }
}

function Get-ReviewManifestPath {
  Get-BenchmarkReviewManifestPath `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath {
    param($InvalidPath)

    Add-Failure "invalid path: $InvalidPath"
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


function Test-ManifestArtifact {
  param(
    [object[]]$Entries,
    [string]$ArtifactName,
    [ValidateSet("required", "comparison", "target")]
    [string]$Kind = "required"
  )

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $ArtifactName `
    -Kind $Kind `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $AddValidationFailure `
    -OnInvalidPath {
    param($InvalidPath)

    Add-Failure "invalid path: $InvalidPath"
  }
}


$ManifestPath = Get-ReviewManifestPath

if (!(Test-SafePathExists -Path $ManifestPath)) {
  Add-Failure "low-gap review manifest was not found: $ManifestPath"
  $ManifestEntries = @()
}
else {
  $ManifestEntries = @(
    Read-BenchmarkReviewManifestEntries `
      -Path $ManifestPath `
      -OnMalformedArtifactLine {
      param($Line)

      Add-Failure "manifest has malformed artifact line: $Line"
    }
  )
}

if ($Failures.Count -gt 0) {
  Fail-Validation -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
}

if ($ManifestEntries.Count -eq 0) {
  Add-Failure "manifest did not contain any artifact entries: $ManifestPath"
  Fail-Validation -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
}

$BenchmarkOutputEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.BenchmarkOutput
$DebugOutputEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.DebugOutput
$SummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.SummaryCsv
$WorkloadsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.WorkloadsCsv
$CountOnlySummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadSummary
$CountOnlyNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadNotes
$LowGapEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowSelectivityGapCsv
$RegressionNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapRegressionNotes
$TrialDetailsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialDetails
$TrialSummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialSummary
$TrialNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialNotes
$WorkloadDetailsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapWorkloadTrialDetails
$WorkloadSummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapWorkloadTrialSummary
$TargetDetailsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialDetails -Kind "target"
$TargetSummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialSummary -Kind "target"
$TargetNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialNotes -Kind "target"
$ReviewNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapReviewNotes
$ReviewManifestEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.LowGapReviewManifest

$AggregateComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.AggregateTrialComparisonCsv -Kind "comparison"
$WorkloadComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.WorkloadTrialComparisonCsv -Kind "comparison"
$CountOnlyComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonCsv -Kind "comparison"
$TargetComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonCsv -Kind "comparison"

Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.AggregateTrialComparisonNotes -Kind "comparison" | Out-Null
Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.WorkloadTrialComparisonNotes -Kind "comparison" | Out-Null
Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonNotes -Kind "comparison" | Out-Null
Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonNotes -Kind "comparison" | Out-Null

Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $BenchmarkOutputEntry -Description "benchmark output"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $DebugOutputEntry -Description "debug output"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $CountOnlyNotesEntry -Description "count-only workload notes"
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
    "mean_candidate_ratio"
  ) `
    -MinimumRows 2 `
    -RequireBaselines $true `
    -TrialCountColumn "trial_count" `
    -ExpectedTrials $ExpectedTrials
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

  Invoke-BenchmarkReviewPolicyCsvValidation -OnFailure $AddValidationFailure `
    -Entry $TargetComparisonEntry `
    -Description "target workload trial comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "previous_workload_name",
    "current_workload_name",
    "previous_mean_timing_ratio",
    "current_mean_timing_ratio",
    "timing_classification"
  ) `
    -MinimumRows 2 `
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