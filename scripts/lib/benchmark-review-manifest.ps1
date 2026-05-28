$BenchmarkReviewManifestStateFound = "found"
$BenchmarkReviewManifestStateMissing = "missing"
$BenchmarkReviewManifestStateSkipped = "skipped"

$BenchmarkReviewArtifactNames = [ordered]@{
  BenchmarkOutput                    = "benchmark output"
  DebugOutput                        = "debug output"
  SummaryCsv                         = "summary CSV"
  WorkloadsCsv                       = "workloads CSV"
  CountOnlyWorkloadSummary           = "count-only workload summary"
  CountOnlyWorkloadNotes             = "count-only workload notes"
  CountOnlyWorkloadComparisonCsv     = "count-only workload comparison CSV"
  CountOnlyWorkloadComparisonNotes   = "count-only workload comparison notes"
  LowSelectivityGapCsv               = "low-selectivity gap CSV"
  LowGapRegressionNotes              = "low-gap regression notes"
  LowGapTrialDetails                 = "low-gap trial details"
  LowGapTrialSummary                 = "low-gap trial summary"
  LowGapTrialNotes                   = "low-gap trial notes"
  LowGapWorkloadTrialDetails         = "low-gap workload trial details"
  LowGapWorkloadTrialSummary         = "low-gap workload trial summary"
  AggregateTrialComparisonCsv        = "aggregate trial comparison CSV"
  AggregateTrialComparisonNotes      = "aggregate trial comparison notes"
  WorkloadTrialComparisonCsv         = "workload trial comparison CSV"
  WorkloadTrialComparisonNotes       = "workload trial comparison notes"
  TargetWorkloadTrialDetails         = "target workload trial details"
  TargetWorkloadTrialSummary         = "target workload trial summary"
  TargetWorkloadTrialNotes           = "target workload trial notes"
  TargetWorkloadTrialComparisonCsv   = "target workload trial comparison CSV"
  TargetWorkloadTrialComparisonNotes = "target workload trial comparison notes"
  LowGapReviewNotes                  = "low-gap review notes"
  LowGapReviewManifest               = "low-gap review manifest"

}

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

function Assert-BenchmarkReviewPreflight {
  param(
    [string]$Label,
    [int]$Trials,
    [string]$TargetWorkloadName,
    [bool]$CleanupFlatArtifacts,
    [bool]$CopyArtifactsToRunFolder
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
    SmallBenchmarkScript                 = Join-Path $ScriptDirectory "run-small-benchmark.ps1"
    LowGapTrialsScript                   = Join-Path $ScriptDirectory "run-low-gap-trials.ps1"
    AggregateComparatorScript            = Join-Path $ScriptDirectory "compare-low-gap-trial-summary.ps1"
    WorkloadComparatorScript             = Join-Path $ScriptDirectory "compare-low-gap-workload-summary.ps1"
    CountOnlyComparatorScript            = Join-Path $ScriptDirectory "compare-count-only-workload-summary.ps1"
    TargetSummaryScript                  = Join-Path $ScriptDirectory "summarize-target-workload-trials.ps1"
    TargetComparatorScript               = Join-Path $ScriptDirectory "compare-target-workload-summary.ps1"
    ArtifactOrganizerScript              = Join-Path $ScriptDirectory "organize-benchmark-artifacts.ps1"
    ArtifactValidatorScript              = Join-Path $ScriptDirectory "validate-benchmark-review-artifacts.ps1"
    FlatArtifactCleanupScript            = Join-Path $ScriptDirectory "cleanup-flat-benchmark-artifacts.ps1"
    OrganizedRunRoot                     = $OrganizedRunRoot
    OrganizedRunDirectory                = Join-Path $OrganizedRunRoot $Label
    PreviousOrganizedRunDirectory        = $PreviousOrganizedRunDirectory
    ReviewNotesPath                      = Join-Path $OutputDir "low-gap-review-notes-$Label.txt"
    ReviewManifestPath                   = Join-Path $OutputDir "low-gap-review-manifest-$Label.txt"
    CurrentTrialSummaryCsv               = Join-Path $OutputDir "low-gap-trial-summary-$Label.csv"
    CurrentWorkloadSummaryCsv            = Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv"
    CurrentCountOnlyWorkloadSummaryCsv   = Join-Path $OutputDir "count-only-workload-summary-$Label.csv"
    CurrentCountOnlyWorkloadSummaryNotes = Join-Path $OutputDir "count-only-workload-summary-$Label.txt"
    CurrentCountOnlyComparisonCsv        = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.csv"
    CurrentCountOnlyComparisonNotes      = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.txt"
    CurrentTargetDetailCsv               = Join-Path $OutputDir "target-workload-trial-details-$Label.csv"
    CurrentTargetSummaryCsv              = Join-Path $OutputDir "target-workload-trial-summary-$Label.csv"
    CurrentTargetNotesPath               = Join-Path $OutputDir "target-workload-trial-notes-$Label.txt"
    PreviousLowSelectivityGapDefaultPath = Join-Path $OutputDir "low-selectivity-gap-$PreviousLabel.csv"
    PreviousTrialSummaryDefaultPath      = Join-Path $OutputDir "low-gap-trial-summary-$PreviousLabel.csv"
    PreviousWorkloadSummaryDefaultPath   = Join-Path $OutputDir "low-gap-workload-trial-summary-$PreviousLabel.csv"
    PreviousCountOnlySummaryDefaultPath  = Join-Path $OutputDir "count-only-workload-summary-$PreviousLabel.csv"
    PreviousTargetSummaryDefaultPath     = Join-Path $OutputDir "target-workload-trial-summary-$PreviousLabel.csv"
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
    $PathSet.CountOnlyComparatorScript
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

function Initialize-BenchmarkReviewNotes {
  param(
    [string]$ReviewNotesPath,
    [string]$Label,
    [string]$PreviousLabel,
    [string]$Dataset,
    [int]$MaxDepth,
    [int]$Trials,
    [int]$Iterations,
    [string]$OutputDir,
    [string]$OrganizedRunRoot,
    [string]$PreviousOrganizedRunDirectory,
    [double]$NoiseThreshold,
    [bool]$SkipTargetWorkloadReview,
    [string]$TargetWorkloadName,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$ForceOrganizedArtifacts,
    [bool]$ValidateArtifacts,
    [bool]$RequireValidatedComparisons,
    [bool]$CleanupFlatArtifacts,
    [string]$PreviousLowSelectivityGapCsv,
    [string]$PreviousTrialSummaryCsv,
    [string]$PreviousWorkloadSummaryCsv,
    [string]$PreviousCountOnlySummaryCsv,
    [string]$PreviousTargetSummaryCsv,
    [object]$PreviousLowSelectivityGapInput,
    [object]$PreviousTrialSummaryInput,
    [object]$PreviousWorkloadSummaryInput,
    [object]$PreviousCountOnlySummaryInput,
    [object]$PreviousTargetSummaryInput
  )

  Set-Utf8Text -Path $ReviewNotesPath -Text "Low-gap benchmark review`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "========================`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Label: $Label`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous label: $PreviousLabel`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Max depth: $MaxDepth`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Trials: $Trials`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Iterations: $Iterations`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Output directory: $OutputDir`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Organized run root: $OrganizedRunRoot`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous organized run directory: $PreviousOrganizedRunDirectory`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Target workload review skipped: $SkipTargetWorkloadReview`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Target workload name: $TargetWorkloadName`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Copy artifacts to run folder: $CopyArtifactsToRunFolder`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Force organized artifacts: $ForceOrganizedArtifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Validate artifacts: $ValidateArtifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Require validated comparisons: $RequireValidatedComparisons`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Cleanup flat artifacts: $CleanupFlatArtifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous low-selectivity gap CSV: $PreviousLowSelectivityGapCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous aggregate trial summary CSV: $PreviousTrialSummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous workload trial summary CSV: $PreviousWorkloadSummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous count-only workload summary CSV: $PreviousCountOnlySummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous target workload summary CSV: $PreviousTargetSummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"

  Add-BenchmarkReviewPreviousInputStatusLines `
    -ReviewNotesPath $ReviewNotesPath `
    -SingleRunInput $PreviousLowSelectivityGapInput `
    -AggregateInput $PreviousTrialSummaryInput `
    -WorkloadInput $PreviousWorkloadSummaryInput `
    -TargetInput $PreviousTargetSummaryInput

  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary status: $($PreviousCountOnlySummaryInput.Status)`r`n"

  if ($PreviousCountOnlySummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary resolved path: $($PreviousCountOnlySummaryInput.ResolvedPath)`r`n"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
}

function Write-BenchmarkReviewManifestHeader {
  param(
    [string]$ManifestPath,
    [string]$Label,
    [string]$PreviousLabel,
    [string]$Dataset,
    [string]$OrganizedRunRoot,
    [string]$PreviousOrganizedRunDirectory,
    [string]$MaxDepth,
    [int]$Trials,
    [int]$Iterations,
    [string]$TargetWorkloadName,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$ForceOrganizedArtifacts,
    [bool]$ValidateArtifacts,
    [bool]$RequireValidatedComparisons,
    [bool]$CleanupFlatArtifacts,
    [string]$NoiseThresholdText
  )

  Set-Utf8Text -Path $ManifestPath -Text "Low-gap review artifact manifest`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "================================`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Label: $Label`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Previous label: $PreviousLabel`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Organized run root: $OrganizedRunRoot`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Previous organized run directory: $PreviousOrganizedRunDirectory`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Max depth: $MaxDepth`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Trials: $Trials`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Iterations: $Iterations`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Target workload: $TargetWorkloadName`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Copy artifacts to run folder: $CopyArtifactsToRunFolder`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Force organized artifacts: $ForceOrganizedArtifacts`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Validate artifacts: $ValidateArtifacts`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Require validated comparisons: $RequireValidatedComparisons`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Cleanup flat artifacts: $CleanupFlatArtifacts`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Noise threshold: +/- $NoiseThresholdText`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "state | artifact | path | note`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "------|----------|------|-----`r`n"
}

function Add-BenchmarkReviewManifestInterpretation {
  param(
    [string]$ManifestPath
  )

  Add-Utf8Text -Path $ManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Manifest interpretation`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "-----------------------`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "found: the artifact exists after the review run`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "missing: the artifact was expected but was not found`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "skipped: the artifact was intentionally not generated because an input or flag made that step unavailable`r`n"
}


function Get-BenchmarkReviewComparisonSkipReason {
  param(
    [string]$PreviousPath,
    [string]$CurrentPath,
    [string]$PreviousDescription,
    [string]$CurrentDescription,
    [bool]$Disabled = $false,
    [string]$DisabledReason = ""
  )

  if ($Disabled) {
    if ([string]::IsNullOrWhiteSpace($DisabledReason)) {
      return "$CurrentDescription was disabled"
    }

    return $DisabledReason
  }

  if ([string]::IsNullOrWhiteSpace($PreviousPath)) {
    return "$PreviousDescription was not provided"
  }

  if (!(Test-BenchmarkReviewPathExists -Path $PreviousPath)) {
    return "$PreviousDescription was not found"
  }

  if (!(Test-BenchmarkReviewPathExists -Path $CurrentPath)) {
    return "$CurrentDescription was not found"
  }

  return ""
}


function Invoke-BenchmarkReviewComparisonStep {
  param(
    [string]$ReviewNotesPath,
    [string]$HostTitle,
    [string]$ReviewTitle,
    [string]$ReviewUnderline,
    [string]$PreviousPath,
    [string]$CurrentPath,
    [string]$PreviousDescription,
    [string]$CurrentDescription,
    [string]$ScriptPath,
    [hashtable]$ScriptArguments,
    [string]$FailureMessage,
    [string[]]$CompletedLines,
    [bool]$Disabled = $false,
    [string]$DisabledConsoleReason = "",
    [string]$DisabledReviewReason = ""
  )

  Write-Host ""
  Write-Host "Running $HostTitle"

  Add-Utf8Text -Path $ReviewNotesPath -Text "$ReviewTitle`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "$ReviewUnderline`r`n"

  $SkipReason = Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $PreviousPath `
    -CurrentPath $CurrentPath `
    -PreviousDescription $PreviousDescription `
    -CurrentDescription $CurrentDescription `
    -Disabled $Disabled `
    -DisabledReason $DisabledConsoleReason

  if (![string]::IsNullOrWhiteSpace($SkipReason)) {
    Write-Host "  skipped: $SkipReason"

    if (!$Disabled -and ![string]::IsNullOrWhiteSpace($PreviousPath) -and !(Test-BenchmarkReviewPathExists -Path $PreviousPath)) {
      Write-Host "  $PreviousPath"
    }
    elseif (!$Disabled -and ![string]::IsNullOrWhiteSpace($PreviousPath) -and (Test-BenchmarkReviewPathExists -Path $PreviousPath) -and !(Test-BenchmarkReviewPathExists -Path $CurrentPath)) {
      Write-Host "  $CurrentPath"
    }

    $ReviewReason = $SkipReason

    if ($Disabled -and ![string]::IsNullOrWhiteSpace($DisabledReviewReason)) {
      $ReviewReason = $DisabledReviewReason
    }

    Add-Utf8Text -Path $ReviewNotesPath -Text "status: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "reason: $ReviewReason`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"

    return
  }

  & $ScriptPath @ScriptArguments

  if ($LASTEXITCODE -ne 0) {
    throw "$FailureMessage $LASTEXITCODE"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "status: completed`r`n"

  foreach ($CompletedLine in $CompletedLines) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "$CompletedLine`r`n"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
}




function Invoke-BenchmarkReviewValidationStep {
  param(
    [string]$ReviewNotesPath,
    [string]$Label,
    [string]$OutputDir,
    [int]$ExpectedTrials,
    [bool]$RequireValidatedComparisons,
    [bool]$AllowSkippedTargetWorkloadReview,
    [string]$ArtifactValidatorScript
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Review artifact validation`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "--------------------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "status: running`r`n"

  Write-Host ""
  Write-Host "Validating review artifacts"
  Write-Host "  label: $Label"

  $ValidatorArguments = @{
    Label          = $Label
    OutputDir      = $OutputDir
    ExpectedTrials = $ExpectedTrials
  }

  if ($RequireValidatedComparisons) {
    $ValidatorArguments["RequireComparisons"] = $true
  }

  if ($AllowSkippedTargetWorkloadReview) {
    $ValidatorArguments["AllowSkippedTargetWorkloadReview"] = $true
  }

  & $ArtifactValidatorScript @ValidatorArguments

  if ($LASTEXITCODE -ne 0) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "status: failed`r`n"
    throw "review artifact validation failed with exit code $LASTEXITCODE"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "status: completed`r`n"
}


function Add-BenchmarkReviewArtifactSummary {
  param(
    [string]$ReviewNotesPath,
    [string]$OutputDir,
    [string]$Label,
    [string]$ReviewManifestPath,
    [string]$TargetWorkloadName
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Review artifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-----------------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "benchmark output: $(Join-Path $OutputDir "benchmark-output-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "debug output: $(Join-Path $OutputDir "debug-output-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "summary CSV: $(Join-Path $OutputDir "summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "workloads CSV: $(Join-Path $OutputDir "workloads-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary: $(Join-Path $OutputDir "count-only-workload-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload notes: $(Join-Path $OutputDir "count-only-workload-summary-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload comparison CSV: $(Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload comparison notes: $(Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-selectivity gap CSV: $(Join-Path $OutputDir "low-selectivity-gap-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap regression notes: $(Join-Path $OutputDir "low-gap-regression-notes-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap trial details: $(Join-Path $OutputDir "low-gap-trial-details-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap trial summary: $(Join-Path $OutputDir "low-gap-trial-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap trial notes: $(Join-Path $OutputDir "low-gap-trial-notes-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap workload trial details: $(Join-Path $OutputDir "low-gap-workload-trial-details-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap workload trial summary: $(Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial details: $(Join-Path $OutputDir "target-workload-trial-details-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial summary: $(Join-Path $OutputDir "target-workload-trial-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial notes: $(Join-Path $OutputDir "target-workload-trial-notes-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial comparison CSV: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial comparison notes: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap review notes: $ReviewNotesPath`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap review manifest: $ReviewManifestPath`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Decision guidance`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-----------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use this review script for query-performance commits.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use aggregate comparison artifacts for low-selectivity bucket movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use workload comparison artifacts for weakest-workload movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use count-only workload comparison artifacts for count-only query mode movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use target-workload comparison artifacts for $TargetWorkloadName movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Do not accept a performance commit based on one noisy single-run result.`r`n"
}


function Add-BenchmarkReviewComparisonAvailabilitySummary {
  param(
    [string]$ReviewNotesPath,
    [object]$PreviousLowSelectivityGapInput,
    [object]$PreviousTrialSummaryInput,
    [string]$CurrentTrialSummaryCsv,
    [object]$PreviousWorkloadSummaryInput,
    [string]$CurrentWorkloadSummaryCsv,
    [object]$PreviousCountOnlySummaryInput,
    [string]$CurrentCountOnlyWorkloadSummaryCsv,
    [bool]$SkipTargetWorkloadReview,
    [object]$PreviousTargetSummaryInput,
    [string]$CurrentTargetSummaryCsv
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Comparison availability summary`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-------------------------------`r`n"

  if ($PreviousLowSelectivityGapInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-gap comparison: completed`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-gap comparison: unavailable`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-gap comparison reason: previous low-selectivity gap CSV was $($PreviousLowSelectivityGapInput.Status)`r`n"
  }

  if ($PreviousTrialSummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentTrialSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison: completed`r`n"
  }
  elseif (!$PreviousTrialSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison reason: previous aggregate trial summary was $($PreviousTrialSummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison reason: current aggregate trial summary was missing`r`n"
  }

  if ($PreviousWorkloadSummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentWorkloadSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison: completed`r`n"
  }
  elseif (!$PreviousWorkloadSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison reason: previous workload trial summary was $($PreviousWorkloadSummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison reason: current workload trial summary was missing`r`n"
  }

  if ($PreviousCountOnlySummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentCountOnlyWorkloadSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison: completed`r`n"
  }
  elseif (!$PreviousCountOnlySummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison reason: previous count-only workload summary was $($PreviousCountOnlySummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison reason: current count-only workload summary was missing`r`n"
  }

  if ($SkipTargetWorkloadReview) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary reason: target workload review was disabled`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison reason: target workload review was disabled`r`n"
    return
  }

  if (Test-BenchmarkReviewPathExists -Path $CurrentTargetSummaryCsv) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary: completed`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary: unavailable`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary reason: current target workload summary was missing`r`n"
  }

  if ($PreviousTargetSummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentTargetSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: completed`r`n"
  }
  elseif (!$PreviousTargetSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison reason: previous target workload summary was $($PreviousTargetSummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison reason: current target workload summary was missing`r`n"
  }
}



function Get-BenchmarkReviewArtifactState {
  param(
    [string]$Path,
    [bool]$Skipped
  )

  if ($Skipped) {
    return $BenchmarkReviewManifestStateSkipped
  }

  if (Test-Path -LiteralPath $Path) {
    return $BenchmarkReviewManifestStateFound
  }

  return $BenchmarkReviewManifestStateMissing
}

function Add-BenchmarkReviewManifestArtifactLine {
  param(
    [string]$ManifestPath,
    [string]$Name,
    [string]$Path,
    [bool]$Skipped,
    [string]$Note = ""
  )

  $State = Get-BenchmarkReviewArtifactState -Path $Path -Skipped $Skipped

  if ([string]::IsNullOrWhiteSpace($Note)) {
    Add-Utf8Text -Path $ManifestPath -Text "$State | $Name | $Path`r`n"
  }
  else {
    Add-Utf8Text -Path $ManifestPath -Text "$State | $Name | $Path | $Note`r`n"
  }
}


function New-BenchmarkReviewManifestArtifact {
  param(
    [string]$Name,
    [string]$Path,
    [bool]$Skipped,
    [string]$Note = ""
  )

  return [PSCustomObject]@{
    Name    = $Name
    Path    = $Path
    Skipped = $Skipped
    Note    = $Note
  }
}

function Add-BenchmarkReviewManifestArtifacts {
  param(
    [string]$ManifestPath,
    [object[]]$Artifacts
  )

  foreach ($Artifact in $Artifacts) {
    Add-BenchmarkReviewManifestArtifactLine `
      -ManifestPath $ManifestPath `
      -Name $Artifact.Name `
      -Path $Artifact.Path `
      -Skipped ([bool]$Artifact.Skipped) `
      -Note $Artifact.Note
  }
}

function Test-BenchmarkReviewManifestArtifactLine {
  param(
    [string]$Line
  )

  return (
    $Line.StartsWith("$BenchmarkReviewManifestStateFound |") -or
    $Line.StartsWith("$BenchmarkReviewManifestStateMissing |") -or
    $Line.StartsWith("$BenchmarkReviewManifestStateSkipped |")
  )
}

function ConvertTo-BenchmarkReviewManifestEntry {
  param(
    [string]$Line,
    [scriptblock]$OnMalformedArtifactLine = $null
  )

  $TrimmedLine = $Line.Trim()

  if (!(Test-BenchmarkReviewManifestArtifactLine -Line $TrimmedLine)) {
    return $null
  }

  $Parts = @(
    $Line -split "\|", 4 |
    ForEach-Object { $_.Trim() }
  )

  if ($Parts.Count -lt 3) {
    if ($null -ne $OnMalformedArtifactLine) {
      & $OnMalformedArtifactLine $Line
    }

    return $null
  }

  $Note = ""

  if ($Parts.Count -ge 4) {
    $Note = $Parts[3]
  }

  return [PSCustomObject]@{
    State    = $Parts[0]
    Artifact = $Parts[1]
    Path     = $Parts[2]
    Note     = $Note
  }
}

function Read-BenchmarkReviewManifestEntries {
  param(
    [string]$Path,
    [scriptblock]$OnMalformedArtifactLine = $null
  )

  $Entries = @()

  if (!(Test-Path -LiteralPath $Path)) {
    return $Entries
  }

  $Lines = Get-Content -LiteralPath $Path

  foreach ($Line in $Lines) {
    $Entry = ConvertTo-BenchmarkReviewManifestEntry `
      -Line $Line `
      -OnMalformedArtifactLine $OnMalformedArtifactLine

    if ($null -ne $Entry) {
      $Entries += $Entry
    }
  }

  return $Entries
}

function Get-BenchmarkReviewManifestEntry {
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
function Test-BenchmarkReviewPathExists {
  param(
    [string]$Path,
    [scriptblock]$OnInvalidPath = $null
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $false
  }

  try {
    return Test-Path -LiteralPath $Path
  }
  catch {
    if ($null -ne $OnInvalidPath) {
      & $OnInvalidPath $Path
    }

    return $false
  }
}

function Resolve-BenchmarkReviewArtifactPath {
  param(
    [string]$Path,
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnInvalidPath = $null
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $Path
  }

  if (
    Test-BenchmarkReviewPathExists `
      -Path $Path `
      -OnInvalidPath $OnInvalidPath
  ) {
    return (Resolve-Path -LiteralPath $Path).Path
  }

  $FileName = Split-Path -Leaf $Path

  if ([string]::IsNullOrWhiteSpace($FileName)) {
    return $Path
  }

  $OrganizedRunDirectory = Join-Path (Join-Path (Join-Path $OutputDir "runs") $Label) ""
  $OrganizedPath = Join-Path $OrganizedRunDirectory $FileName

  if (
    Test-BenchmarkReviewPathExists `
      -Path $OrganizedPath `
      -OnInvalidPath $OnInvalidPath
  ) {
    return (Resolve-Path -LiteralPath $OrganizedPath).Path
  }

  return $Path
}

function Get-BenchmarkReviewManifestPath {
  param(
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnInvalidPath = $null
  )

  $FlatManifestPath = Join-Path $OutputDir "low-gap-review-manifest-$Label.txt"
  $ResolvedFlatManifestPath = Resolve-BenchmarkReviewArtifactPath `
    -Path $FlatManifestPath `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath $OnInvalidPath

  if (
    Test-BenchmarkReviewPathExists `
      -Path $ResolvedFlatManifestPath `
      -OnInvalidPath $OnInvalidPath
  ) {
    return (Resolve-Path -LiteralPath $ResolvedFlatManifestPath).Path
  }

  return $FlatManifestPath
}

function Get-BenchmarkReviewManifestLoadResult {
  param(
    [string]$OutputDir,
    [string]$Label,
    [bool]$RequireComparisons = $false,
    [bool]$AllowSkippedTargetWorkloadReview = $false,
    [scriptblock]$OnFailure,
    [scriptblock]$OnInvalidPath = $null
  )

  $ManifestPath = Get-BenchmarkReviewManifestPath `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath $OnInvalidPath

  if (
    !(
      Test-BenchmarkReviewPathExists `
        -Path $ManifestPath `
        -OnInvalidPath $OnInvalidPath
    )
  ) {
    if ($null -ne $OnFailure) {
      & $OnFailure "low-gap review manifest was not found: $ManifestPath"
    }

    return [pscustomobject]@{
      ManifestPath        = $ManifestPath
      Entries             = @()
      ResolvedArtifacts   = $null
      HasBlockingFailures = $true
    }
  }

  $MalformedArtifactLines = [System.Collections.Generic.List[string]]::new()

  $ManifestEntries = @(
    Read-BenchmarkReviewManifestEntries `
      -Path $ManifestPath `
      -OnMalformedArtifactLine {
      param($Line)

      $MalformedArtifactLines.Add($Line) | Out-Null

      if ($null -ne $OnFailure) {
        & $OnFailure "manifest has malformed artifact line: $Line"
      }
    }
  )

  if ($MalformedArtifactLines.Count -gt 0) {
    return [pscustomobject]@{
      ManifestPath        = $ManifestPath
      Entries             = $ManifestEntries
      ResolvedArtifacts   = $null
      HasBlockingFailures = $true
    }
  }

  if ($ManifestEntries.Count -eq 0) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest did not contain any artifact entries: $ManifestPath"
    }

    return [pscustomobject]@{
      ManifestPath        = $ManifestPath
      Entries             = $ManifestEntries
      ResolvedArtifacts   = $null
      HasBlockingFailures = $true
    }
  }

  $ResolvedArtifacts = Resolve-BenchmarkReviewManifestArtifactSet `
    -Entries $ManifestEntries `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  return [pscustomobject]@{
    ManifestPath        = $ManifestPath
    Entries             = $ManifestEntries
    ResolvedArtifacts   = $ResolvedArtifacts
    HasBlockingFailures = $false
  }
}


function Resolve-BenchmarkReviewManifestArtifact {
  param(
    [object[]]$Entries,
    [string]$ArtifactName,
    [ValidateSet("required", "comparison", "target")]
    [string]$Kind = "required",
    [bool]$RequireComparisons = $false,
    [bool]$AllowSkippedTargetWorkloadReview = $false,
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnFailure,
    [scriptblock]$OnInvalidPath = $null
  )

  $Entry = Get-BenchmarkReviewManifestEntry -Entries $Entries -ArtifactName $ArtifactName

  if ($null -eq $Entry) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest does not list required artifact: $ArtifactName"
    }

    return $null
  }

  if ($Entry.State -eq $BenchmarkReviewManifestStateMissing) {
    if ($Kind -eq "comparison" -and !$RequireComparisons) {
      return $Entry
    }

    if ($null -ne $OnFailure) {
      & $OnFailure "manifest reports missing artifact '$ArtifactName': $($Entry.Path)"
    }

    return $Entry
  }

  if ($Entry.State -eq $BenchmarkReviewManifestStateSkipped) {
    if ($Kind -eq "comparison" -and !$RequireComparisons) {
      return $Entry
    }

    if ($Kind -eq "target" -and $AllowSkippedTargetWorkloadReview) {
      return $Entry
    }

    if ($null -ne $OnFailure) {
      & $OnFailure "manifest reports skipped artifact '$ArtifactName': $($Entry.Note)"
    }

    return $Entry
  }

  if ($Entry.State -ne $BenchmarkReviewManifestStateFound) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest has unknown state '$($Entry.State)' for artifact '$ArtifactName'"
    }

    return $Entry
  }

  $ResolvedPath = Resolve-BenchmarkReviewArtifactPath `
    -Path $Entry.Path `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath $OnInvalidPath

  if (
    !(
      Test-BenchmarkReviewPathExists `
        -Path $ResolvedPath `
        -OnInvalidPath $OnInvalidPath
    )
  ) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest reports found artifact '$ArtifactName' but file was not found: $($Entry.Path)"
    }

    return $Entry
  }

  $Entry.Path = $ResolvedPath

  return $Entry
}


function Resolve-BenchmarkReviewManifestArtifactSet {
  param(
    [object[]]$Entries,
    [bool]$RequireComparisons = $false,
    [bool]$AllowSkippedTargetWorkloadReview = $false,
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnFailure,
    [scriptblock]$OnInvalidPath = $null
  )

  $ResolvedArtifacts = [ordered]@{}

  $ResolvedArtifacts.BenchmarkOutput = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.BenchmarkOutput `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.DebugOutput = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.DebugOutput `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.Summary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.SummaryCsv `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.Workloads = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.WorkloadsCsv `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.CountOnlySummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.CountOnlyNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.LowGap = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowSelectivityGapCsv `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.RegressionNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapRegressionNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TrialDetails = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialDetails `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TrialSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TrialNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.WorkloadDetails = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapWorkloadTrialDetails `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.WorkloadSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapWorkloadTrialSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetDetails = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialDetails `
    -Kind "target" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialSummary `
    -Kind "target" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialNotes `
    -Kind "target" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.ReviewNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapReviewNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.ReviewManifest = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapReviewManifest `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.AggregateComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.AggregateTrialComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.WorkloadComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.WorkloadTrialComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.CountOnlyComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.AggregateTrialComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.WorkloadTrialComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  return [PSCustomObject]$ResolvedArtifacts
}


function Test-BenchmarkReviewNonEmptyFile {
  param(
    [object]$Entry,
    [string]$Description,
    [scriptblock]$OnFailure
  )

  if ($null -eq $Entry) {
    return
  }

  if ($Entry.State -ne $BenchmarkReviewManifestStateFound) {
    return
  }

  if (!(Test-BenchmarkReviewPathExists -Path $Entry.Path)) {
    & $OnFailure "$Description was not found: $($Entry.Path)"
    return
  }

  $Item = Get-Item -LiteralPath $Entry.Path

  if ($Item.Length -le 0) {
    & $OnFailure "$Description is empty: $($Entry.Path)"
  }
}

function Read-BenchmarkReviewValidatedCsv {
  param(
    [object]$Entry,
    [string]$Description,
    [string[]]$RequiredColumns,
    [int]$MinimumRows = 1,
    [scriptblock]$OnFailure
  )

  if ($null -eq $Entry) {
    return @()
  }

  if ($Entry.State -ne $BenchmarkReviewManifestStateFound) {
    return @()
  }

  if (!(Test-BenchmarkReviewPathExists -Path $Entry.Path)) {
    & $OnFailure "$Description was not found: $($Entry.Path)"
    return @()
  }

  try {
    $Rows = @(Import-Csv -LiteralPath $Entry.Path)
  }
  catch {
    & $OnFailure "$Description could not be parsed as CSV: $($Entry.Path)"
    & $OnFailure "  $($_.Exception.Message)"
    return @()
  }

  if ($Rows.Count -lt $MinimumRows) {
    & $OnFailure "$Description has fewer rows than expected. expected at least $MinimumRows, found $($Rows.Count): $($Entry.Path)"
    return $Rows
  }

  $Columns = @(
    $Rows[0].PSObject.Properties |
    ForEach-Object { $_.Name }
  )

  foreach ($ColumnName in $RequiredColumns) {
    if ($Columns -notcontains $ColumnName) {
      & $OnFailure "$Description is missing required column '$ColumnName': $($Entry.Path)"
    }
  }

  return $Rows
}

function Test-BenchmarkReviewBooleanColumnAllTrue {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string]$Description,
    [scriptblock]$OnFailure
  )

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue) {
      & $OnFailure "$Description has missing value in column '$ColumnName'"
      continue
    }

    $TextValue = $RawValue.ToString().Trim().ToLowerInvariant()

    if ($TextValue -ne "true") {
      & $OnFailure "$Description has non-true value in column '$ColumnName': $RawValue"
    }
  }
}

function Test-BenchmarkReviewRequiredBaselines {
  param(
    [object[]]$Rows,
    [string]$Description,
    [scriptblock]$OnFailure
  )

  $BaselineNames = @(
    $Rows |
    ForEach-Object { $_.baseline_name } |
    Where-Object { ![string]::IsNullOrWhiteSpace($_) } |
    Sort-Object -Unique
  )

  foreach ($BaselineName in @("kd_tree", "r_tree")) {
    if ($BaselineNames -notcontains $BaselineName) {
      & $OnFailure "$Description does not contain required baseline row: $BaselineName"
    }
  }
}

function Test-BenchmarkReviewExpectedTrialCount {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string]$Description,
    [int]$ExpectedTrials,
    [scriptblock]$OnFailure
  )

  if ($ExpectedTrials -le 0) {
    return
  }

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue -or [string]::IsNullOrWhiteSpace($RawValue.ToString())) {
      & $OnFailure "$Description has missing trial count in column '$ColumnName'"
      continue
    }

    try {
      $TrialCount = [int][double]::Parse(
        $RawValue.ToString(),
        [System.Globalization.CultureInfo]::InvariantCulture
      )

      if ($TrialCount -ne $ExpectedTrials) {
        & $OnFailure "$Description expected trial count $ExpectedTrials but found $TrialCount"
      }
    }
    catch {
      & $OnFailure "$Description has invalid trial count value '$RawValue'"
    }
  }
}

function Read-BenchmarkReviewPolicyValidatedCsv {
  param(
    [object]$Entry,
    [string]$Description,
    [string[]]$RequiredColumns,
    [int]$MinimumRows = 1,
    [string[]]$BooleanTrueColumns = @(),
    [bool]$RequireBaselines = $false,
    [string]$TrialCountColumn = "",
    [int]$ExpectedTrials = 0,
    [scriptblock]$OnFailure
  )

  $Rows = Read-BenchmarkReviewValidatedCsv `
    -Entry $Entry `
    -Description $Description `
    -RequiredColumns $RequiredColumns `
    -MinimumRows $MinimumRows `
    -OnFailure $OnFailure

  foreach ($ColumnName in $BooleanTrueColumns) {
    Test-BenchmarkReviewBooleanColumnAllTrue `
      -Rows $Rows `
      -ColumnName $ColumnName `
      -Description $Description `
      -OnFailure $OnFailure
  }

  if ($RequireBaselines) {
    Test-BenchmarkReviewRequiredBaselines `
      -Rows $Rows `
      -Description $Description `
      -OnFailure $OnFailure
  }

  if (![string]::IsNullOrWhiteSpace($TrialCountColumn)) {
    Test-BenchmarkReviewExpectedTrialCount `
      -Rows $Rows `
      -ColumnName $TrialCountColumn `
      -Description $Description `
      -ExpectedTrials $ExpectedTrials `
      -OnFailure $OnFailure
  }

  return $Rows
}




function Invoke-BenchmarkReviewPolicyCsvValidation {
  param(
    [object]$Entry,
    [string]$Description,
    [string[]]$RequiredColumns,
    [int]$MinimumRows = 1,
    [string[]]$BooleanTrueColumns = @(),
    [bool]$RequireBaselines = $false,
    [string]$TrialCountColumn = "",
    [int]$ExpectedTrials = 0,
    [scriptblock]$OnFailure
  )

  Read-BenchmarkReviewPolicyValidatedCsv `
    -Entry $Entry `
    -Description $Description `
    -RequiredColumns $RequiredColumns `
    -MinimumRows $MinimumRows `
    -BooleanTrueColumns $BooleanTrueColumns `
    -RequireBaselines $RequireBaselines `
    -TrialCountColumn $TrialCountColumn `
    -ExpectedTrials $ExpectedTrials `
    -OnFailure $OnFailure | Out-Null
}

function Copy-BenchmarkReviewNotesToOrganizedRun {
  param(
    [string]$ReviewNotesPath,
    [string]$OrganizedRunDirectory,
    [bool]$CopyArtifactsToRunFolder
  )

  if (!$CopyArtifactsToRunFolder) {
    return
  }

  New-Item -ItemType Directory -Force -Path $OrganizedRunDirectory | Out-Null

  Copy-Item `
    -LiteralPath $ReviewNotesPath `
    -Destination (Join-Path $OrganizedRunDirectory (Split-Path -Leaf $ReviewNotesPath)) `
    -Force
}

function Invoke-BenchmarkReviewOrganizedArtifactCopy {
  param(
    [string]$ReviewNotesPath,
    [string]$OutputDir,
    [string]$Label,
    [string]$OrganizedRunDirectory,
    [bool]$ForceOrganizedArtifacts,
    [string]$ArtifactOrganizerScript
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Organized artifact copy`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-----------------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "status: running`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "destination: $OrganizedRunDirectory`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "force: $ForceOrganizedArtifacts`r`n"

  Write-Host ""
  Write-Host "Copying current review artifacts to organized run folder"
  Write-Host "  label: $Label"
  Write-Host "  destination: $OrganizedRunDirectory"

  $OrganizerArguments = @{
    ArtifactRoot = $OutputDir
    Label        = $Label
    Copy         = $true
  }

  if ($ForceOrganizedArtifacts) {
    $OrganizerArguments["Force"] = $true
  }

  & $ArtifactOrganizerScript @OrganizerArguments

  if ($LASTEXITCODE -ne 0) {
    throw "organized artifact copy failed with exit code $LASTEXITCODE"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "status: completed`r`n"

  Copy-BenchmarkReviewNotesToOrganizedRun `
    -ReviewNotesPath $ReviewNotesPath `
    -OrganizedRunDirectory $OrganizedRunDirectory `
    -CopyArtifactsToRunFolder $true
}


function Write-BenchmarkReviewValidationReport {
  param(
    [string]$Label,
    [string]$OutputDir,
    [string]$ManifestPath,
    [int]$ExpectedTrials,
    [bool]$RequireComparisons,
    [bool]$AllowSkippedTargetWorkloadReview,
    [int]$ManifestEntryCount,
    [string[]]$Failures = @()
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

function Write-BenchmarkReviewCompletionOutput {
  param(
    [string]$ReviewNotesPath,
    [string]$ReviewManifestPath,
    [string]$OrganizedRunDirectory,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$CleanupFlatArtifacts
  )

  Write-Host ""
  Write-Host "Low-gap review complete:"

  if ($CleanupFlatArtifacts -and $CopyArtifactsToRunFolder) {
    Write-Host "  organized review notes: $(Join-Path $OrganizedRunDirectory (Split-Path -Leaf $ReviewNotesPath))"
    Write-Host "  organized review manifest: $(Join-Path $OrganizedRunDirectory (Split-Path -Leaf $ReviewManifestPath))"
    Write-Host "  organized run folder: $OrganizedRunDirectory"
  }
  else {
    Write-Host "  $ReviewNotesPath"
    Write-Host "  $ReviewManifestPath"

    if ($CopyArtifactsToRunFolder) {
      Write-Host "  organized run folder: $OrganizedRunDirectory"
    }
  }
}

