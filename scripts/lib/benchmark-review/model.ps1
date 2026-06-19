Set-Variable -Name BenchmarkReviewManifestStateFound -Value "found"
Set-Variable -Name BenchmarkReviewManifestStateMissing -Value "missing"
Set-Variable -Name BenchmarkReviewManifestStateSkipped -Value "skipped"

Set-Variable -Name BenchmarkReviewArtifactNames -Value ([ordered]@{
    BenchmarkOutput                           = "benchmark output"
    DebugOutput                               = "debug output"
    SummaryCsv                                = "summary CSV"
    WorkloadsCsv                              = "workloads CSV"
    CountOnlyWorkloadSummary                  = "count-only workload summary"
    CountOnlyWorkloadNotes                    = "count-only workload notes"
    CountOnlyWorkloadComparisonCsv            = "count-only workload comparison CSV"
    CountOnlyWorkloadComparisonNotes          = "count-only workload comparison notes"
    MaterializationModeSummary                = "materialization mode summary"
    MaterializationModeNotes                  = "materialization mode notes"
    TypedArchiveLoadSummary                   = "typed archive load summary"
    TypedArchiveLoadNotes                     = "typed archive load notes"
    TypedArchiveAppendRebuildSummary          = "typed archive append rebuild summary"
    TypedArchiveAppendRebuildNotes            = "typed archive append rebuild notes"
    TypedArchiveCompactionSummary             = "typed archive compaction summary"
    TypedArchiveCompactionNotes               = "typed archive compaction notes"
    TypedArchiveMaintenanceSummary            = "typed archive maintenance summary"
    TypedArchiveMaintenanceNotes              = "typed archive maintenance notes"
    TypedArchiveAppendDeltaMaintenanceSummary = "typed archive append-delta maintenance summary"
    TypedArchiveAppendDeltaMaintenanceNotes   = "typed archive append-delta maintenance notes"
    TypedQueryIndexArchive                    = "typed query index archive"
    MaterializationModeComparisonCsv          = "materialization mode comparison CSV"
    MaterializationModeComparisonNotes        = "materialization mode comparison notes"
    LowSelectivityGapCsv                      = "low-selectivity gap CSV"
    LowGapRegressionNotes                     = "low-gap regression notes"
    LowGapTrialDetails                        = "low-gap trial details"
    LowGapTrialSummary                        = "low-gap trial summary"
    LowGapTrialNotes                          = "low-gap trial notes"
    LowGapWorkloadTrialDetails                = "low-gap workload trial details"
    LowGapWorkloadTrialSummary                = "low-gap workload trial summary"
    AggregateTrialComparisonCsv               = "aggregate trial comparison CSV"
    AggregateTrialComparisonNotes             = "aggregate trial comparison notes"
    WorkloadTrialComparisonCsv                = "workload trial comparison CSV"
    WorkloadTrialComparisonNotes              = "workload trial comparison notes"
    TargetWorkloadTrialDetails                = "target workload trial details"
    TargetWorkloadTrialSummary                = "target workload trial summary"
    TargetWorkloadTrialNotes                  = "target workload trial notes"
    TargetWorkloadTrialComparisonCsv          = "target workload trial comparison CSV"
    TargetWorkloadTrialComparisonNotes        = "target workload trial comparison notes"
    LowGapReviewNotes                         = "low-gap review notes"
    LowGapReviewManifest                      = "low-gap review manifest"

  })

function Resolve-BenchmarkReviewTargetWorkloadName {
  param(
    [string]$TargetWorkloadName,
    [string]$Dataset
  )

  if (![string]::IsNullOrWhiteSpace($TargetWorkloadName)) {
    return $TargetWorkloadName
  }

  if ($Dataset -eq "large") {
    return "large_cross_cluster_boundary"
  }

  return "cluster_boundary_range"
}

function Resolve-BenchmarkReviewEffectiveMaxDepth {
  param(
    [int]$MaxDepth,
    [string]$Dataset
  )

  if ($MaxDepth -gt 0) {
    return $MaxDepth
  }

  if ($Dataset -eq "large") {
    return 16
  }

  return 8
}

function Format-BenchmarkReviewRunId {
  param(
    [int]$RunNumber
  )

  return "run{0:d3}" -f $RunNumber
}

function Format-BenchmarkReviewAttemptId {
  param(
    [int]$RunIndex
  )

  return "r{0:d2}" -f $RunIndex
}

function Normalize-BenchmarkReviewRunTopic {
  param(
    [string]$RunTopic
  )

  $Normalized = $RunTopic.Trim().ToLowerInvariant()
  $Normalized = $Normalized -replace "[^a-z0-9]+", "-"
  $Normalized = $Normalized -replace "^-+", ""
  $Normalized = $Normalized -replace "-+$", ""
  $Normalized = $Normalized -replace "-+", "-"

  return $Normalized
}

function Resolve-BenchmarkReviewLabel {
  param(
    [string]$Label,
    [int]$RunNumber,
    [int]$RunIndex,
    [string]$RunTopic,
    [string]$Dataset
  )

  if (![string]::IsNullOrWhiteSpace($Label)) {
    return $Label
  }

  if ($RunNumber -eq 0 -and [string]::IsNullOrWhiteSpace($RunTopic)) {
    return "local"
  }

  if ($RunNumber -lt 1) {
    throw "`-RunNumber` must be at least 1 when deriving a benchmark label"
  }

  if ($RunIndex -lt 1) {
    throw "`-RunIndex` must be at least 1 when deriving a benchmark label"
  }

  if ([string]::IsNullOrWhiteSpace($RunTopic)) {
    throw "`-RunTopic` cannot be empty when deriving a benchmark label"
  }

  $NormalizedTopic = Normalize-BenchmarkReviewRunTopic -RunTopic $RunTopic

  if ([string]::IsNullOrWhiteSpace($NormalizedTopic)) {
    throw "`-RunTopic` must contain at least one alphanumeric character"
  }

  $RunId = Format-BenchmarkReviewRunId -RunNumber $RunNumber
  $AttemptId = Format-BenchmarkReviewAttemptId -RunIndex $RunIndex

  return "$RunId-$AttemptId-$NormalizedTopic-$Dataset"
}

function Assert-BenchmarkReviewPreflight {
  param(
    [string]$Label,
    [int]$Trials,
    [string]$TargetWorkloadName,
    [bool]$CleanupFlatArtifacts,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$UpdateHistory,
    [bool]$ValidateArtifacts,
    [int]$RunNumber,
    [string]$RunTopic
  )

  if ([string]::IsNullOrWhiteSpace($Label)) {
    throw "`-Label` cannot be empty"
  }

  if ($Trials -lt 1) {
    throw "`-Trials` must be at least 1"
  }

  if ([string]::IsNullOrWhiteSpace($TargetWorkloadName)) {
    throw "`-TargetWorkloadName` could not be resolved"
  }

  if ($CleanupFlatArtifacts -and !$CopyArtifactsToRunFolder) {
    throw "-CleanupFlatArtifacts requires -CopyArtifactsToRunFolder so flat artifacts are not removed before an organized run copy exists"
  }

  if ($UpdateHistory -and !$CopyArtifactsToRunFolder) {
    throw "-UpdateHistory requires -CopyArtifactsToRunFolder so history rows point at preserved raw artifacts"
  }

  if ($UpdateHistory -and !$ValidateArtifacts) {
    throw "-UpdateHistory requires -ValidateArtifacts so only validated review runs enter history"
  }

  if ($UpdateHistory -and $RunNumber -lt 1) {
    throw "-UpdateHistory requires -RunNumber so history rows use run-number nomenclature"
  }

  if ($UpdateHistory -and [string]::IsNullOrWhiteSpace($RunTopic)) {
    throw "-UpdateHistory requires -RunTopic so history rows describe the benchmark topic"
  }
}

function Assert-BenchmarkReviewScript {
  param(
    [string]$Path
  )

  if (!(Test-Path -LiteralPath $Path)) {
    throw "required script was not found: $Path"
  }
}

function Assert-BenchmarkReviewScripts {
  param(
    [string[]]$Paths
  )

  foreach ($Path in $Paths) {
    Assert-BenchmarkReviewScript -Path $Path
  }
}

function New-BenchmarkReviewPathSet {
  param(
    [string]$ScriptDirectory,
    [string]$OutputDir,
    [string]$Label,
    [string]$PreviousLabel
  )

  $OrganizedRunRoot = Join-Path $OutputDir "runs"
  $PreviousOrganizedRunDirectory = Get-BenchmarkReviewPreviousOrganizedRunDirectory `
    -PreviousLabel $PreviousLabel `
    -OrganizedRunRoot $OrganizedRunRoot

  return [PSCustomObject]@{
    SmallBenchmarkScript                                  = Join-Path (Join-Path $ScriptDirectory "run") "run-small-benchmark.ps1"
    LowGapTrialsScript                                    = Join-Path (Join-Path $ScriptDirectory "run") "run-low-gap-trials.ps1"
    AggregateComparatorScript                             = Join-Path (Join-Path $ScriptDirectory "compare") "compare-low-gap-trial-summary.ps1"
    WorkloadComparatorScript                              = Join-Path (Join-Path $ScriptDirectory "compare") "compare-low-gap-workload-summary.ps1"
    CountOnlyComparatorScript                             = Join-Path (Join-Path $ScriptDirectory "compare") "compare-count-only-workload-summary.ps1"
    MaterializationComparatorScript                       = Join-Path (Join-Path $ScriptDirectory "compare") "compare-materialization-mode-summary.ps1"
    TargetSummaryScript                                   = Join-Path (Join-Path $ScriptDirectory "summarize") "summarize-target-workload-trials.ps1"
    TargetComparatorScript                                = Join-Path (Join-Path $ScriptDirectory "compare") "compare-target-workload-summary.ps1"
    ArtifactOrganizerScript                               = Join-Path (Join-Path $ScriptDirectory "artifacts") "organize-benchmark-artifacts.ps1"
    ArtifactValidatorScript                               = Join-Path (Join-Path $ScriptDirectory "review") "validate-benchmark-review-artifacts.ps1"
    FlatArtifactCleanupScript                             = Join-Path (Join-Path $ScriptDirectory "artifacts") "cleanup-flat-benchmark-artifacts.ps1"
    OrganizedRunRoot                                      = $OrganizedRunRoot
    OrganizedRunDirectory                                 = Join-Path $OrganizedRunRoot $Label
    PreviousOrganizedRunDirectory                         = $PreviousOrganizedRunDirectory
    ReviewNotesPath                                       = Join-Path $OutputDir "low-gap-review-notes-$Label.txt"
    ReviewManifestPath                                    = Join-Path $OutputDir "low-gap-review-manifest-$Label.txt"
    CurrentTrialSummaryCsv                                = Join-Path $OutputDir "low-gap-trial-summary-$Label.csv"
    CurrentWorkloadSummaryCsv                             = Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv"
    CurrentCountOnlyWorkloadSummaryCsv                    = Join-Path $OutputDir "count-only-workload-summary-$Label.csv"
    CurrentCountOnlyWorkloadSummaryNotes                  = Join-Path $OutputDir "count-only-workload-summary-$Label.txt"
    CurrentCountOnlyComparisonCsv                         = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.csv"
    CurrentCountOnlyComparisonNotes                       = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.txt"
    CurrentMaterializationSummaryCsv                      = Join-Path $OutputDir "materialization-mode-summary-$Label.csv"
    CurrentMaterializationSummaryNotes                    = Join-Path $OutputDir "materialization-mode-summary-$Label.txt"
    CurrentTypedArchiveLoadSummaryCsv                     = Join-Path $OutputDir "typed-archive-load-summary-$Label.csv"
    CurrentTypedArchiveLoadSummaryNotes                   = Join-Path $OutputDir "typed-archive-load-summary-$Label.txt"
    CurrentTypedArchiveAppendRebuildSummaryCsv            = Join-Path $OutputDir "typed-archive-append-rebuild-summary-$Label.csv"
    CurrentTypedArchiveAppendRebuildSummaryNotes          = Join-Path $OutputDir "typed-archive-append-rebuild-summary-$Label.txt"
    CurrentTypedArchiveCompactionSummaryCsv               = Join-Path $OutputDir "typed-archive-compaction-summary-$Label.csv"
    CurrentTypedArchiveCompactionSummaryNotes             = Join-Path $OutputDir "typed-archive-compaction-summary-$Label.txt"
    CurrentTypedArchiveMaintenanceSummaryCsv              = Join-Path $OutputDir "typed-archive-maintenance-summary-$Label.csv"
    CurrentTypedArchiveMaintenanceSummaryNotes            = Join-Path $OutputDir "typed-archive-maintenance-summary-$Label.txt"
    CurrentTypedArchiveAppendDeltaMaintenanceSummaryCsv   = Join-Path $OutputDir "typed-archive-append-delta-maintenance-summary-$Label.csv"
    CurrentTypedArchiveAppendDeltaMaintenanceSummaryNotes = Join-Path $OutputDir "typed-archive-append-delta-maintenance-summary-$Label.txt"
    CurrentTypedQueryIndexArchive                         = Join-Path $OutputDir "typed-query-index-archive-$Label.fse"
    CurrentMaterializationComparisonCsv                   = Join-Path $OutputDir "materialization-mode-summary-comparison-$Label.csv"
    CurrentMaterializationComparisonNotes                 = Join-Path $OutputDir "materialization-mode-summary-comparison-$Label.txt"
    CurrentTargetDetailCsv                                = Join-Path $OutputDir "target-workload-trial-details-$Label.csv"
    CurrentTargetSummaryCsv                               = Join-Path $OutputDir "target-workload-trial-summary-$Label.csv"
    CurrentTargetNotesPath                                = Join-Path $OutputDir "target-workload-trial-notes-$Label.txt"
    PreviousLowSelectivityGapDefaultPath                  = Join-Path $OutputDir "low-selectivity-gap-$PreviousLabel.csv"
    PreviousTrialSummaryDefaultPath                       = Join-Path $OutputDir "low-gap-trial-summary-$PreviousLabel.csv"
    PreviousWorkloadSummaryDefaultPath                    = Join-Path $OutputDir "low-gap-workload-trial-summary-$PreviousLabel.csv"
    PreviousCountOnlySummaryDefaultPath                   = Join-Path $OutputDir "count-only-workload-summary-$PreviousLabel.csv"
    PreviousMaterializationSummaryDefaultPath             = Join-Path $OutputDir "materialization-mode-summary-$PreviousLabel.csv"
    PreviousTargetSummaryDefaultPath                      = Join-Path $OutputDir "target-workload-trial-summary-$PreviousLabel.csv"
  }
}

function Get-BenchmarkReviewRequiredScripts {
  param(
    [object]$PathSet,
    [bool]$SkipTargetWorkloadReview,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$ValidateArtifacts,
    [bool]$CleanupFlatArtifacts
  )

  $RequiredScripts = @(
    $PathSet.SmallBenchmarkScript,
    $PathSet.LowGapTrialsScript,
    $PathSet.AggregateComparatorScript,
    $PathSet.WorkloadComparatorScript,
    $PathSet.CountOnlyComparatorScript,
    $PathSet.MaterializationComparatorScript
  )

  if (!$SkipTargetWorkloadReview) {
    $RequiredScripts += @(
      $PathSet.TargetSummaryScript,
      $PathSet.TargetComparatorScript
    )
  }

  if ($CopyArtifactsToRunFolder) {
    $RequiredScripts += $PathSet.ArtifactOrganizerScript
  }

  if ($ValidateArtifacts) {
    $RequiredScripts += $PathSet.ArtifactValidatorScript
  }

  if ($CleanupFlatArtifacts) {
    $RequiredScripts += $PathSet.FlatArtifactCleanupScript
  }

  return $RequiredScripts
}

function Get-BenchmarkReviewPreviousOrganizedRunDirectory {
  param(
    [string]$PreviousLabel,
    [string]$OrganizedRunRoot
  )

  if ([string]::IsNullOrWhiteSpace($PreviousLabel)) {
    return ""
  }

  return Join-Path $OrganizedRunRoot $PreviousLabel
}

function Resolve-BenchmarkReviewOptionalPreviousPath {
  param(
    [string]$ExplicitPath,
    [string]$DefaultPath,
    [string]$PreviousLabel,
    [string]$PreviousOrganizedRunDirectory
  )

  if (![string]::IsNullOrWhiteSpace($ExplicitPath)) {
    return $ExplicitPath
  }

  if ([string]::IsNullOrWhiteSpace($PreviousLabel)) {
    return ""
  }

  if (Test-Path -LiteralPath $DefaultPath) {
    return $DefaultPath
  }

  if (![string]::IsNullOrWhiteSpace($PreviousOrganizedRunDirectory)) {
    $FileName = Split-Path -Leaf $DefaultPath
    $OrganizedPath = Join-Path $PreviousOrganizedRunDirectory $FileName

    if (Test-Path -LiteralPath $OrganizedPath) {
      return $OrganizedPath
    }
  }

  return $DefaultPath
}

function Get-BenchmarkReviewInputStatus {
  param(
    [string]$Path
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return [PSCustomObject]@{
      Status       = "not provided"
      Exists       = $false
      ResolvedPath = ""
    }
  }

  if (!(Test-Path -LiteralPath $Path)) {
    return [PSCustomObject]@{
      Status       = "path not found"
      Exists       = $false
      ResolvedPath = ""
    }
  }

  return [PSCustomObject]@{
    Status       = "found"
    Exists       = $true
    ResolvedPath = (Resolve-Path -LiteralPath $Path).Path
  }
}

function Add-BenchmarkReviewPreviousInputStatusLines {
  param(
    [string]$ReviewNotesPath,
    [object]$SingleRunInput,
    [object]$AggregateInput,
    [object]$WorkloadInput,
    [object]$TargetInput
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous input status`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "---------------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-selectivity CSV status: $($SingleRunInput.Status)`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate trial summary status: $($AggregateInput.Status)`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "workload trial summary status: $($WorkloadInput.Status)`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial summary status: $($TargetInput.Status)`r`n"

  if ($SingleRunInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-selectivity CSV resolved path: $($SingleRunInput.ResolvedPath)`r`n"
  }

  if ($AggregateInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate trial summary resolved path: $($AggregateInput.ResolvedPath)`r`n"
  }

  if ($WorkloadInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload trial summary resolved path: $($WorkloadInput.ResolvedPath)`r`n"
  }

  if ($TargetInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial summary resolved path: $($TargetInput.ResolvedPath)`r`n"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
}
