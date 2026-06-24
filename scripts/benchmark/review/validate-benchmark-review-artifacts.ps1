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


function Test-BenchmarkReviewTextColumnAllInSet {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string[]]$ExpectedValues,
    [string]$Description
  )

  $ExpectedSet = @{}

  foreach ($ExpectedValue in $ExpectedValues) {
    $ExpectedSet[$ExpectedValue.Trim().ToLowerInvariant()] = $true
  }

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue) {
      Add-Failure "$Description has missing value in column '$ColumnName'"
      continue
    }

    $TextValue = $RawValue.ToString().Trim().ToLowerInvariant()

    if (!$ExpectedSet.ContainsKey($TextValue)) {
      Add-Failure "$Description has unexpected value in column '$ColumnName': $RawValue"
    }
  }
}


function Test-BenchmarkReviewDelimitedTextColumnAllInSet {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string[]]$ExpectedValues,
    [string]$Delimiter,
    [string]$Description
  )

  $ExpectedSet = @{}

  foreach ($ExpectedValue in $ExpectedValues) {
    $ExpectedSet[$ExpectedValue.Trim().ToLowerInvariant()] = $true
  }

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue) {
      Add-Failure "$Description has missing value in column '$ColumnName'"
      continue
    }

    $TextValue = $RawValue.ToString().Trim().ToLowerInvariant()

    if ([string]::IsNullOrWhiteSpace($TextValue)) {
      Add-Failure "$Description has empty value in column '$ColumnName'"
      continue
    }

    if ($ExpectedSet.ContainsKey($TextValue)) {
      continue
    }

    $Parts = @($TextValue.Split($Delimiter) | ForEach-Object { $_.Trim() })

    foreach ($Part in $Parts) {
      if ([string]::IsNullOrWhiteSpace($Part)) {
        Add-Failure "$Description has empty delimited value in column '$ColumnName': $RawValue"
        continue
      }

      if (!$ExpectedSet.ContainsKey($Part)) {
        Add-Failure "$Description has unexpected delimited value in column '$ColumnName': $RawValue"
      }
    }
  }
}


function Test-BenchmarkReviewTextColumnsAllEqual {
  param(
    [object[]]$Rows,
    [string]$LeftColumnName,
    [string]$RightColumnName,
    [string]$Description
  )

  foreach ($Row in $Rows) {
    $LeftRawValue = $Row.$LeftColumnName
    $RightRawValue = $Row.$RightColumnName

    if ($null -eq $LeftRawValue) {
      Add-Failure "$Description has missing value in column '$LeftColumnName'"
      continue
    }

    if ($null -eq $RightRawValue) {
      Add-Failure "$Description has missing value in column '$RightColumnName'"
      continue
    }

    $LeftTextValue = $LeftRawValue.ToString().Trim()
    $RightTextValue = $RightRawValue.ToString().Trim()

    if ($LeftTextValue -ne $RightTextValue) {
      Add-Failure "$Description has mismatched values for '$LeftColumnName' and '$RightColumnName'. left '$LeftTextValue', right '$RightTextValue'"
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
$TypedIndexedComparisonSummaryEntry = $ResolvedArtifacts.TypedIndexedComparisonSummary
$TypedIndexedComparisonNotesEntry = $ResolvedArtifacts.TypedIndexedComparisonNotes
$TypedArchiveLoadSummaryEntry = $ResolvedArtifacts.TypedArchiveLoadSummary
$TypedArchiveLoadNotesEntry = $ResolvedArtifacts.TypedArchiveLoadNotes
$TypedArchiveAppendRebuildSummaryEntry = $ResolvedArtifacts.TypedArchiveAppendRebuildSummary
$TypedArchiveAppendRebuildNotesEntry = $ResolvedArtifacts.TypedArchiveAppendRebuildNotes
$TypedArchiveAppendDeltaQuerySummaryEntry = $ResolvedArtifacts.TypedArchiveAppendDeltaQuerySummary
$TypedArchiveAppendDeltaQueryNotesEntry = $ResolvedArtifacts.TypedArchiveAppendDeltaQueryNotes
$TypedArchiveCompactionSummaryEntry = $ResolvedArtifacts.TypedArchiveCompactionSummary
$TypedArchiveCompactionNotesEntry = $ResolvedArtifacts.TypedArchiveCompactionNotes
$TypedArchiveMaintenanceSummaryEntry = $ResolvedArtifacts.TypedArchiveMaintenanceSummary
$TypedArchiveMaintenanceNotesEntry = $ResolvedArtifacts.TypedArchiveMaintenanceNotes
$TypedArchiveAppendDeltaMaintenanceSummaryEntry = $ResolvedArtifacts.TypedArchiveAppendDeltaMaintenanceSummary
$TypedArchiveAppendDeltaMaintenanceNotesEntry = $ResolvedArtifacts.TypedArchiveAppendDeltaMaintenanceNotes
$TypedQueryIndexArchiveEntry = $ResolvedArtifacts.TypedQueryIndexArchive
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
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedIndexedComparisonNotesEntry -Description "typed indexed comparison notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedArchiveLoadNotesEntry -Description "typed archive load notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedArchiveAppendRebuildNotesEntry -Description "typed archive append rebuild notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedArchiveAppendDeltaQueryNotesEntry -Description "typed archive append-delta query notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedArchiveCompactionNotesEntry -Description "typed archive compaction notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedArchiveMaintenanceNotesEntry -Description "typed archive maintenance notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedArchiveAppendDeltaMaintenanceNotesEntry -Description "typed archive append-delta maintenance notes"
Test-BenchmarkReviewNonEmptyFile -OnFailure $AddValidationFailure -Entry $TypedQueryIndexArchiveEntry -Description "typed query index archive"
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
  "visitor_elapsed",
  "row_view_elapsed",
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

$TypedIndexedComparisonSummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $TypedIndexedComparisonSummaryEntry `
  -Description "typed indexed comparison summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "typed_scan_elapsed",
  "indexed_typed_elapsed",
  "timing_ratio",
  "candidate_ratio",
  "planner_strategy",
  "selectivity_bucket",
  "planner_risk",
  "planner_predicate_evaluation_delta",
  "planner_flat_scan_record_delta",
  "planner_traversal_node_visit_delta",
  "record_avoidance_ratio",
  "agreement"
) `
  -MinimumRows 1

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $TypedIndexedComparisonSummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "typed indexed comparison summary"

Test-BenchmarkReviewTextColumnAllInSet `
  -Rows $TypedIndexedComparisonSummaryRows `
  -ColumnName "planner_strategy" `
  -ExpectedValues @("fsetraversal", "flatscan", "hybrid", "noop") `
  -Description "typed indexed comparison summary"

Test-BenchmarkReviewTextColumnAllInSet `
  -Rows $TypedIndexedComparisonSummaryRows `
  -ColumnName "selectivity_bucket" `
  -ExpectedValues @("empty", "selective", "moderate", "broad") `
  -Description "typed indexed comparison summary"

Test-BenchmarkReviewDelimitedTextColumnAllInSet `
  -Rows $TypedIndexedComparisonSummaryRows `
  -ColumnName "planner_risk" `
  -ExpectedValues @("none", "broad", "materialization", "high_dimensional_low_constraint", "append_delta") `
  -Delimiter "+" `
  -Description "typed indexed comparison summary"

$TypedArchiveLoadSummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $TypedArchiveLoadSummaryEntry `
  -Description "typed archive load summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "archive_bytes",
  "in_memory_elapsed",
  "warm_loaded_elapsed",
  "cold_loaded_elapsed",
  "warm_loaded_to_in_memory_ratio",
  "cold_loaded_to_in_memory_ratio",
  "cold_loaded_to_warm_loaded_ratio",
  "agreement"
) `
  -MinimumRows 1 `
  -OnFailure $AddValidationFailure

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $TypedArchiveLoadSummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "typed archive load summary"

$TypedArchiveAppendRebuildSummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $TypedArchiveAppendRebuildSummaryEntry `
  -Description "typed archive append rebuild summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "base_record_count",
  "appended_record_count",
  "resulting_record_count",
  "archive_bytes_before_append",
  "archive_bytes_after_append",
  "archive_byte_growth",
  "matched_records_after_append",
  "append_rebuild_elapsed",
  "agreement"
) `
  -MinimumRows 1 `
  -OnFailure $AddValidationFailure

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $TypedArchiveAppendRebuildSummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "typed archive append rebuild summary"

$TypedArchiveAppendDeltaQuerySummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $TypedArchiveAppendDeltaQuerySummaryEntry `
  -Description "typed archive append-delta query summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "base_record_count",
  "appended_record_count",
  "rebuilt_record_count",
  "append_delta_matched_records",
  "rebuilt_matched_records",
  "append_delta_reconstructed_records",
  "rebuilt_reconstructed_records",
  "append_delta_candidate_ratio",
  "rebuilt_candidate_ratio",
  "rebuilt_to_append_delta_ratio",
  "rebuilt_query_elapsed",
  "append_delta_query_elapsed",
  "agreement"
) `
  -MinimumRows 1 `
  -OnFailure $AddValidationFailure

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $TypedArchiveAppendDeltaQuerySummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "typed archive append-delta query summary"

Test-BenchmarkReviewTextColumnsAllEqual `
  -Rows $TypedArchiveAppendDeltaQuerySummaryRows `
  -LeftColumnName "append_delta_matched_records" `
  -RightColumnName "rebuilt_matched_records" `
  -Description "typed archive append-delta query summary"

$TypedArchiveCompactionSummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $TypedArchiveCompactionSummaryEntry `
  -Description "typed archive compaction summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "base_record_count",
  "tombstone_count",
  "removed_record_count",
  "retained_record_count",
  "index_bytes_before_compaction",
  "index_bytes_after_compaction",
  "index_byte_delta",
  "tombstone_bytes_before_compaction",
  "tombstone_bytes_after_compaction",
  "tombstone_byte_delta",
  "logical_archive_bytes_before_compaction",
  "logical_archive_bytes_after_compaction",
  "logical_archive_byte_delta",
  "matched_records_after_compaction",
  "compaction_elapsed",
  "agreement"
) `
  -MinimumRows 1 `
  -OnFailure $AddValidationFailure

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $TypedArchiveCompactionSummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "typed archive compaction summary"

$TypedArchiveMaintenanceSummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $TypedArchiveMaintenanceSummaryEntry `
  -Description "typed archive maintenance summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "maintenance_action",
  "maintenance_reason",
  "maintenance_status_requires_archive_write",
  "base_record_count",
  "pending_append_record_count",
  "tombstone_count",
  "resulting_record_count",
  "index_bytes_before_maintenance",
  "index_bytes_after_maintenance",
  "index_byte_delta",
  "tombstone_bytes_before_maintenance",
  "tombstone_bytes_after_maintenance",
  "tombstone_byte_delta",
  "logical_archive_bytes_before_maintenance",
  "logical_archive_bytes_after_maintenance",
  "logical_archive_byte_delta",
  "matched_records_after_maintenance",
  "maintenance_elapsed",
  "agreement"
) `
  -MinimumRows 1 `
  -OnFailure $AddValidationFailure

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $TypedArchiveMaintenanceSummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "typed archive maintenance summary"

Test-BenchmarkReviewTextColumnAllInSet `
  -Rows $TypedArchiveMaintenanceSummaryRows `
  -ColumnName "maintenance_status_requires_archive_write" `
  -ExpectedValues @("true", "false") `
  -Description "typed archive maintenance summary"

$TypedArchiveAppendDeltaMaintenanceSummaryRows = Read-BenchmarkReviewPolicyValidatedCsv `
  -Entry $TypedArchiveAppendDeltaMaintenanceSummaryEntry `
  -Description "typed archive append-delta maintenance summary" `
  -RequiredColumns @(
  "dataset",
  "max_depth",
  "workload_name",
  "maintenance_action",
  "maintenance_reason",
  "maintenance_status_requires_archive_write",
  "base_record_count",
  "pending_append_record_count",
  "tombstone_count",
  "resulting_record_count",
  "index_bytes_before_maintenance",
  "index_bytes_after_maintenance",
  "index_byte_delta",
  "append_bytes_before_maintenance",
  "append_bytes_after_maintenance",
  "append_byte_delta",
  "tombstone_bytes_before_maintenance",
  "tombstone_bytes_after_maintenance",
  "tombstone_byte_delta",
  "logical_archive_bytes_before_maintenance",
  "logical_archive_bytes_after_maintenance",
  "logical_archive_byte_delta",
  "matched_records_after_maintenance",
  "maintenance_elapsed",
  "agreement"
) `
  -MinimumRows 1 `
  -OnFailure $AddValidationFailure

Test-BenchmarkReviewTextColumnAllEquals `
  -Rows $TypedArchiveAppendDeltaMaintenanceSummaryRows `
  -ColumnName "agreement" `
  -ExpectedValue "pass" `
  -Description "typed archive append-delta maintenance summary"

Test-BenchmarkReviewTextColumnAllInSet `
  -Rows $TypedArchiveAppendDeltaMaintenanceSummaryRows `
  -ColumnName "maintenance_status_requires_archive_write" `
  -ExpectedValues @("true", "false") `
  -Description "typed archive append-delta maintenance summary"

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
    "visitor_elapsed_classification",
    "row_view_elapsed_classification",
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
