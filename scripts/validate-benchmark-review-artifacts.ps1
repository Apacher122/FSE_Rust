param(
  [string]$Label = "local",
  [string]$OutputDir = "benchmark_artifacts",
  [int]$ExpectedTrials = 0,
  [switch]$RequireComparisons,
  [switch]$AllowSkippedTargetWorkloadReview
)

$ErrorActionPreference = "Stop"

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

function Test-SafePathExists {
  param(
    [string]$Path
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $false
  }

  try {
    return Test-Path -LiteralPath $Path
  }
  catch {
    Add-Failure "invalid path: $Path"
    return $false
  }
}

function Resolve-ReviewArtifactPath {
  param(
    [string]$Path
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $Path
  }

  if (Test-SafePathExists -Path $Path) {
    return (Resolve-Path -LiteralPath $Path).Path
  }

  $FileName = Split-Path -Leaf $Path

  if ([string]::IsNullOrWhiteSpace($FileName)) {
    return $Path
  }

  $OrganizedRunDirectory = Join-Path (Join-Path (Join-Path $OutputDir "runs") $Label) ""
  $OrganizedPath = Join-Path $OrganizedRunDirectory $FileName

  if (Test-SafePathExists -Path $OrganizedPath) {
    return (Resolve-Path -LiteralPath $OrganizedPath).Path
  }

  return $Path
}

function Get-ReviewManifestPath {
  $FlatManifestPath = Join-Path $OutputDir "low-gap-review-manifest-$Label.txt"
  $ResolvedFlatManifestPath = Resolve-ReviewArtifactPath -Path $FlatManifestPath

  if (Test-SafePathExists -Path $ResolvedFlatManifestPath) {
    return (Resolve-Path -LiteralPath $ResolvedFlatManifestPath).Path
  }

  return $FlatManifestPath
}

function Write-ValidationReport {
  param(
    [string]$ManifestPath,
    [int]$ManifestEntryCount
  )

  Write-Host "Benchmark review artifact validation"
  Write-Host "===================================="
  Write-Host ""
  Write-Host "Label:                     $Label"
  Write-Host "Output dir:                $OutputDir"
  Write-Host "Manifest:                  $ManifestPath"
  Write-Host "Expected trials:           $ExpectedTrials"
  Write-Host "Require comparisons:       $RequireComparisons"
  Write-Host "Allow skipped target:      $AllowSkippedTargetWorkloadReview"
  Write-Host "Manifest artifacts loaded: $ManifestEntryCount"
  Write-Host "Failures:                  $($Failures.Count)"
  Write-Host ""

  if ($Failures.Count -gt 0) {
    Write-Host "Validation failures:"
    Write-Host ""

    foreach ($Failure in $Failures) {
      Write-Host "  $Failure"
    }
  }
}

function Fail-Validation {
  param(
    [string]$ManifestPath,
    [int]$ManifestEntryCount
  )

  Write-ValidationReport -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntryCount
  throw "benchmark review artifact validation failed"
}

function Read-ManifestEntries {
  param(
    [string]$Path
  )

  $Entries = @()

  if (!(Test-SafePathExists -Path $Path)) {
    Add-Failure "low-gap review manifest was not found: $Path"
    return $Entries
  }

  $Lines = Get-Content -LiteralPath $Path

  foreach ($Line in $Lines) {
    $TrimmedLine = $Line.Trim()

    if (
      !$TrimmedLine.StartsWith("found |") -and
      !$TrimmedLine.StartsWith("missing |") -and
      !$TrimmedLine.StartsWith("skipped |")
    ) {
      continue
    }

    $Parts = @(
      $Line -split "\|", 4 |
      ForEach-Object { $_.Trim() }
    )

    if ($Parts.Count -lt 3) {
      Add-Failure "manifest has malformed artifact line: $Line"
      continue
    }

    $Note = ""

    if ($Parts.Count -ge 4) {
      $Note = $Parts[3]
    }

    $Entries += [PSCustomObject]@{
      State    = $Parts[0]
      Artifact = $Parts[1]
      Path     = $Parts[2]
      Note     = $Note
    }
  }

  return $Entries
}

function Get-ManifestEntry {
  param(
    [object[]]$Entries,
    [string]$ArtifactName
  )

  $Matches = @(
    $Entries |
    Where-Object { $_.Artifact -eq $ArtifactName }
  )

  if ($Matches.Count -eq 0) {
    return $null
  }

  return $Matches[0]
}

function Test-ManifestArtifact {
  param(
    [object[]]$Entries,
    [string]$ArtifactName,
    [ValidateSet("required", "comparison", "target")]
    [string]$Kind = "required"
  )

  $Entry = Get-ManifestEntry -Entries $Entries -ArtifactName $ArtifactName

  if ($null -eq $Entry) {
    Add-Failure "manifest does not list required artifact: $ArtifactName"
    return $null
  }

  if ($Entry.State -eq "missing") {
    if ($Kind -eq "comparison" -and !$RequireComparisons) {
      return $Entry
    }

    Add-Failure "manifest reports missing artifact '$ArtifactName': $($Entry.Path)"
    return $Entry
  }

  if ($Entry.State -eq "skipped") {
    if ($Kind -eq "comparison" -and !$RequireComparisons) {
      return $Entry
    }

    if ($Kind -eq "target" -and $AllowSkippedTargetWorkloadReview) {
      return $Entry
    }

    Add-Failure "manifest reports skipped artifact '$ArtifactName': $($Entry.Note)"
    return $Entry
  }

  if ($Entry.State -ne "found") {
    Add-Failure "manifest has unknown state '$($Entry.State)' for artifact '$ArtifactName'"
    return $Entry
  }

  $ResolvedPath = Resolve-ReviewArtifactPath -Path $Entry.Path

  if (!(Test-SafePathExists -Path $ResolvedPath)) {
    Add-Failure "manifest reports found artifact '$ArtifactName' but file was not found: $($Entry.Path)"
    return $Entry
  }

  $Entry.Path = $ResolvedPath

  return $Entry
}

function Test-NonEmptyFile {
  param(
    [object]$Entry,
    [string]$Description
  )

  if ($null -eq $Entry) {
    return
  }

  if ($Entry.State -ne "found") {
    return
  }

  if (!(Test-SafePathExists -Path $Entry.Path)) {
    Add-Failure "$Description was not found: $($Entry.Path)"
    return
  }

  $Item = Get-Item -LiteralPath $Entry.Path

  if ($Item.Length -le 0) {
    Add-Failure "$Description is empty: $($Entry.Path)"
  }
}

function Read-ValidatedCsv {
  param(
    [object]$Entry,
    [string]$Description,
    [string[]]$RequiredColumns,
    [int]$MinimumRows = 1
  )

  if ($null -eq $Entry) {
    return @()
  }

  if ($Entry.State -ne "found") {
    return @()
  }

  if (!(Test-SafePathExists -Path $Entry.Path)) {
    Add-Failure "$Description was not found: $($Entry.Path)"
    return @()
  }

  try {
    $Rows = @(Import-Csv -LiteralPath $Entry.Path)
  }
  catch {
    Add-Failure "$Description could not be parsed as CSV: $($Entry.Path)"
    Add-Failure "  $($_.Exception.Message)"
    return @()
  }

  if ($Rows.Count -lt $MinimumRows) {
    Add-Failure "$Description has fewer rows than expected. expected at least $MinimumRows, found $($Rows.Count): $($Entry.Path)"
    return $Rows
  }

  $Columns = @(
    $Rows[0].PSObject.Properties |
    ForEach-Object { $_.Name }
  )

  foreach ($ColumnName in $RequiredColumns) {
    if ($Columns -notcontains $ColumnName) {
      Add-Failure "$Description is missing required column '$ColumnName': $($Entry.Path)"
    }
  }

  return $Rows
}

function Test-BooleanColumnAllTrue {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string]$Description
  )

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue) {
      Add-Failure "$Description has missing value in column '$ColumnName'"
      continue
    }

    $TextValue = $RawValue.ToString().Trim().ToLowerInvariant()

    if ($TextValue -ne "true") {
      Add-Failure "$Description has non-true value in column '$ColumnName': $RawValue"
    }
  }
}

function Test-RequiredBaselines {
  param(
    [object[]]$Rows,
    [string]$Description
  )

  $BaselineNames = @(
    $Rows |
    ForEach-Object { $_.baseline_name } |
    Where-Object { ![string]::IsNullOrWhiteSpace($_) } |
    Sort-Object -Unique
  )

  foreach ($BaselineName in @("kd_tree", "r_tree")) {
    if ($BaselineNames -notcontains $BaselineName) {
      Add-Failure "$Description does not contain required baseline row: $BaselineName"
    }
  }
}

function Test-ExpectedTrialCount {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string]$Description
  )

  if ($ExpectedTrials -le 0) {
    return
  }

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue -or [string]::IsNullOrWhiteSpace($RawValue.ToString())) {
      Add-Failure "$Description has missing trial count in column '$ColumnName'"
      continue
    }

    try {
      $TrialCount = [int][double]::Parse(
        $RawValue.ToString(),
        [System.Globalization.CultureInfo]::InvariantCulture
      )

      if ($TrialCount -ne $ExpectedTrials) {
        Add-Failure "$Description expected trial count $ExpectedTrials but found $TrialCount"
      }
    }
    catch {
      Add-Failure "$Description has invalid trial count value '$RawValue'"
    }
  }
}

$ManifestPath = Get-ReviewManifestPath
$ManifestEntries = @(Read-ManifestEntries -Path $ManifestPath)

if ($Failures.Count -gt 0) {
  Fail-Validation -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
}

if ($ManifestEntries.Count -eq 0) {
  Add-Failure "manifest did not contain any artifact entries: $ManifestPath"
  Fail-Validation -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
}

$BenchmarkOutputEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "benchmark output"
$DebugOutputEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "debug output"
$SummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "summary CSV"
$WorkloadsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "workloads CSV"
$CountOnlySummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "count-only workload summary"
$CountOnlyNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "count-only workload notes"
$LowGapEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-selectivity gap CSV"
$RegressionNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap regression notes"
$TrialDetailsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap trial details"
$TrialSummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap trial summary"
$TrialNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap trial notes"
$WorkloadDetailsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap workload trial details"
$WorkloadSummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap workload trial summary"
$TargetDetailsEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "target workload trial details" -Kind "target"
$TargetSummaryEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "target workload trial summary" -Kind "target"
$TargetNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "target workload trial notes" -Kind "target"
$ReviewNotesEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap review notes"
$ReviewManifestEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "low-gap review manifest"

$AggregateComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "aggregate trial comparison CSV" -Kind "comparison"
$WorkloadComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "workload trial comparison CSV" -Kind "comparison"
$CountOnlyComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "count-only workload comparison CSV" -Kind "comparison"
$TargetComparisonEntry = Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "target workload trial comparison CSV" -Kind "comparison"

Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "aggregate trial comparison notes" -Kind "comparison" | Out-Null
Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "workload trial comparison notes" -Kind "comparison" | Out-Null
Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "count-only workload comparison notes" -Kind "comparison" | Out-Null
Test-ManifestArtifact -Entries $ManifestEntries -ArtifactName "target workload trial comparison notes" -Kind "comparison" | Out-Null

Test-NonEmptyFile -Entry $BenchmarkOutputEntry -Description "benchmark output"
Test-NonEmptyFile -Entry $DebugOutputEntry -Description "debug output"
Test-NonEmptyFile -Entry $CountOnlyNotesEntry -Description "count-only workload notes"
Test-NonEmptyFile -Entry $RegressionNotesEntry -Description "low-gap regression notes"
Test-NonEmptyFile -Entry $TrialNotesEntry -Description "low-gap trial notes"
Test-NonEmptyFile -Entry $TargetNotesEntry -Description "target workload trial notes"
Test-NonEmptyFile -Entry $ReviewNotesEntry -Description "low-gap review notes"
Test-NonEmptyFile -Entry $ReviewManifestEntry -Description "low-gap review manifest"

$SummaryRows = Read-ValidatedCsv `
  -Entry $SummaryEntry `
  -Description "summary CSV" `
  -RequiredColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "hierarchy_topology_valid",
  "parent_child_bounds_valid",
  "baseline_name"
) `
  -MinimumRows 3

Test-BooleanColumnAllTrue -Rows $SummaryRows -ColumnName "index_valid" -Description "summary CSV"
Test-BooleanColumnAllTrue -Rows $SummaryRows -ColumnName "leaf_cardinality_valid" -Description "summary CSV"
Test-BooleanColumnAllTrue -Rows $SummaryRows -ColumnName "hierarchy_topology_valid" -Description "summary CSV"
Test-BooleanColumnAllTrue -Rows $SummaryRows -ColumnName "parent_child_bounds_valid" -Description "summary CSV"
Test-RequiredBaselines -Rows $SummaryRows -Description "summary CSV"

$WorkloadRows = Read-ValidatedCsv `
  -Entry $WorkloadsEntry `
  -Description "workloads CSV" `
  -RequiredColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "baseline_name",
  "workload_name",
  "count_only_stats_match_owned"
) `
  -MinimumRows 1

Test-BooleanColumnAllTrue -Rows $WorkloadRows -ColumnName "index_valid" -Description "workloads CSV"
Test-BooleanColumnAllTrue -Rows $WorkloadRows -ColumnName "leaf_cardinality_valid" -Description "workloads CSV"
Test-BooleanColumnAllTrue -Rows $WorkloadRows -ColumnName "count_only_stats_match_owned" -Description "workloads CSV"
Test-RequiredBaselines -Rows $WorkloadRows -Description "workloads CSV"

$CountOnlySummaryRows = Read-ValidatedCsv `
  -Entry $CountOnlySummaryEntry `
  -Description "count-only workload summary" `
  -RequiredColumns @(
  "index_valid",
  "leaf_cardinality_valid",
  "baseline_name",
  "selectivity_bucket",
  "all_stats_match_owned"
) `
  -MinimumRows 1

Test-BooleanColumnAllTrue -Rows $CountOnlySummaryRows -ColumnName "index_valid" -Description "count-only workload summary"
Test-BooleanColumnAllTrue -Rows $CountOnlySummaryRows -ColumnName "leaf_cardinality_valid" -Description "count-only workload summary"
Test-BooleanColumnAllTrue -Rows $CountOnlySummaryRows -ColumnName "all_stats_match_owned" -Description "count-only workload summary"
Test-RequiredBaselines -Rows $CountOnlySummaryRows -Description "count-only workload summary"

$LowGapRows = Read-ValidatedCsv `
  -Entry $LowGapEntry `
  -Description "low-selectivity gap CSV" `
  -RequiredColumns @(
  "baseline_name",
  "low_workload_count",
  "low_weighted_candidate_ratio",
  "low_mean_timing_ratio"
) `
  -MinimumRows 2

Test-RequiredBaselines -Rows $LowGapRows -Description "low-selectivity gap CSV"

$TrialSummaryRows = Read-ValidatedCsv `
  -Entry $TrialSummaryEntry `
  -Description "low-gap trial summary" `
  -RequiredColumns @(
  "baseline_name",
  "trial_count",
  "mean_low_mean_timing_ratio",
  "classification"
) `
  -MinimumRows 2

Test-RequiredBaselines -Rows $TrialSummaryRows -Description "low-gap trial summary"
Test-ExpectedTrialCount -Rows $TrialSummaryRows -ColumnName "trial_count" -Description "low-gap trial summary"

$WorkloadSummaryRows = Read-ValidatedCsv `
  -Entry $WorkloadSummaryEntry `
  -Description "low-gap workload trial summary" `
  -RequiredColumns @(
  "baseline_name",
  "trial_count",
  "most_frequent_weakest_workload",
  "mean_weakest_timing_ratio"
) `
  -MinimumRows 2

Test-RequiredBaselines -Rows $WorkloadSummaryRows -Description "low-gap workload trial summary"
Test-ExpectedTrialCount -Rows $WorkloadSummaryRows -ColumnName "trial_count" -Description "low-gap workload trial summary"

if ($null -ne $TargetSummaryEntry -and $TargetSummaryEntry.State -eq "found") {
  $TargetSummaryRows = Read-ValidatedCsv `
    -Entry $TargetSummaryEntry `
    -Description "target workload trial summary" `
    -RequiredColumns @(
    "baseline_name",
    "workload_name",
    "trial_count",
    "mean_timing_ratio",
    "mean_candidate_ratio"
  ) `
    -MinimumRows 2

  Test-RequiredBaselines -Rows $TargetSummaryRows -Description "target workload trial summary"
  Test-ExpectedTrialCount -Rows $TargetSummaryRows -ColumnName "trial_count" -Description "target workload trial summary"
}

if ($RequireComparisons) {
  $AggregateComparisonRows = Read-ValidatedCsv `
    -Entry $AggregateComparisonEntry `
    -Description "aggregate trial comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "previous_mean_low_mean_timing_ratio",
    "current_mean_low_mean_timing_ratio",
    "timing_classification"
  ) `
    -MinimumRows 2

  Test-RequiredBaselines -Rows $AggregateComparisonRows -Description "aggregate trial comparison CSV"

  $WorkloadComparisonRows = Read-ValidatedCsv `
    -Entry $WorkloadComparisonEntry `
    -Description "workload trial comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "previous_mean_weakest_timing_ratio",
    "current_mean_weakest_timing_ratio",
    "timing_classification"
  ) `
    -MinimumRows 2

  Test-RequiredBaselines -Rows $WorkloadComparisonRows -Description "workload trial comparison CSV"

  $CountOnlyComparisonRows = Read-ValidatedCsv `
    -Entry $CountOnlyComparisonEntry `
    -Description "count-only workload comparison CSV" `
    -RequiredColumns @(
    "baseline_name",
    "selectivity_bucket",
    "previous_weighted_count_only_speedup_ratio",
    "current_weighted_count_only_speedup_ratio",
    "weighted_count_only_speedup_classification"
  ) `
    -MinimumRows 1

  Test-RequiredBaselines -Rows $CountOnlyComparisonRows -Description "count-only workload comparison CSV"

  $TargetComparisonRows = Read-ValidatedCsv `
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
    -MinimumRows 2

  Test-RequiredBaselines -Rows $TargetComparisonRows -Description "target workload trial comparison CSV"
}

if ($Failures.Count -gt 0) {
  Fail-Validation -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
}

Write-ValidationReport -ManifestPath $ManifestPath -ManifestEntryCount $ManifestEntries.Count
Write-Host "Benchmark review artifacts are valid."