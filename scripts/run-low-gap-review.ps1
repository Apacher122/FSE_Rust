param(
  [string]$Label = "local",
  [string]$PreviousLabel = "",
  [ValidateSet("small", "large")]
  [string]$Dataset = "small",
  [ValidateRange(0, 2147483647)]
  [int]$MaxDepth = 0,
  [int]$Trials = 5,
  [int]$Iterations = 10000,
  [string]$OutputDir = "benchmark_artifacts",
  [string]$PreviousLowSelectivityGapCsv = "",
  [string]$PreviousTrialSummaryCsv = "",
  [string]$PreviousWorkloadSummaryCsv = "",
  [string]$PreviousCountOnlySummaryCsv = "",
  [string]$TargetWorkloadName = "",
  [string]$PreviousTargetSummaryCsv = "",
  [switch]$SkipTargetWorkloadReview,
  [switch]$CopyArtifactsToRunFolder,
  [switch]$ForceOrganizedArtifacts,
  [switch]$ValidateArtifacts,
  [switch]$RequireValidatedComparisons,
  [switch]$CleanupFlatArtifacts,
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"
$BenchmarkReviewManifestLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-review-manifest.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkReviewManifestLibrary)) {
  throw "benchmark review manifest helper library was not found: $BenchmarkReviewManifestLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkReviewManifestLibrary

if ([string]::IsNullOrWhiteSpace($Label)) {
  throw "`-Label` cannot be empty"
}

if ($Trials -lt 1) {
  throw "`-Trials` must be at least 1"
}

if ([string]::IsNullOrWhiteSpace($TargetWorkloadName)) {
  if ($Dataset -eq "large") {
    $TargetWorkloadName = "large_cross_cluster_boundary"
  }
  else {
    $TargetWorkloadName = "cluster_boundary_range"
  }
}

if ([string]::IsNullOrWhiteSpace($TargetWorkloadName)) {
  throw "`-TargetWorkloadName` could not be resolved"
}

if ($MaxDepth -gt 0) {
  $EffectiveMaxDepth = $MaxDepth
}
elseif ($Dataset -eq "large") {
  $EffectiveMaxDepth = 16
}
else {
  $EffectiveMaxDepth = 8
}

if ($CleanupFlatArtifacts -and !$CopyArtifactsToRunFolder) {
  throw "-CleanupFlatArtifacts requires -CopyArtifactsToRunFolder so flat artifacts are not removed before an organized run copy exists"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$ScriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path

$SmallBenchmarkScript = Join-Path $ScriptDirectory "run-small-benchmark.ps1"
$LowGapTrialsScript = Join-Path $ScriptDirectory "run-low-gap-trials.ps1"
$AggregateComparatorScript = Join-Path $ScriptDirectory "compare-low-gap-trial-summary.ps1"
$WorkloadComparatorScript = Join-Path $ScriptDirectory "compare-low-gap-workload-summary.ps1"
$CountOnlyComparatorScript = Join-Path $ScriptDirectory "compare-count-only-workload-summary.ps1"
$TargetSummaryScript = Join-Path $ScriptDirectory "summarize-target-workload-trials.ps1"
$TargetComparatorScript = Join-Path $ScriptDirectory "compare-target-workload-summary.ps1"
$ArtifactOrganizerScript = Join-Path $ScriptDirectory "organize-benchmark-artifacts.ps1"
$ArtifactValidatorScript = Join-Path $ScriptDirectory "validate-benchmark-review-artifacts.ps1"
$FlatArtifactCleanupScript = Join-Path $ScriptDirectory "cleanup-flat-benchmark-artifacts.ps1"

$OrganizedRunRoot = Join-Path $OutputDir "runs"
$OrganizedRunDirectory = Join-Path $OrganizedRunRoot $Label

$PreviousOrganizedRunDirectory = Get-BenchmarkReviewPreviousOrganizedRunDirectory `
  -PreviousLabel $PreviousLabel `
  -OrganizedRunRoot $OrganizedRunRoot

$ReviewNotesPath = Join-Path $OutputDir "low-gap-review-notes-$Label.txt"
$ReviewManifestPath = Join-Path $OutputDir "low-gap-review-manifest-$Label.txt"
$CurrentTrialSummaryCsv = Join-Path $OutputDir "low-gap-trial-summary-$Label.csv"
$CurrentWorkloadSummaryCsv = Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv"
$CurrentCountOnlyWorkloadSummaryCsv = Join-Path $OutputDir "count-only-workload-summary-$Label.csv"
$CurrentCountOnlyWorkloadSummaryNotes = Join-Path $OutputDir "count-only-workload-summary-$Label.txt"
$CurrentCountOnlyComparisonCsv = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.csv"
$CurrentCountOnlyComparisonNotes = Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.txt"
$CurrentTargetDetailCsv = Join-Path $OutputDir "target-workload-trial-details-$Label.csv"
$CurrentTargetSummaryCsv = Join-Path $OutputDir "target-workload-trial-summary-$Label.csv"
$CurrentTargetNotesPath = Join-Path $OutputDir "target-workload-trial-notes-$Label.txt"


function Require-Script {
  param(
    [string]$Path
  )

  if (!(Test-Path -LiteralPath $Path)) {
    throw "required script was not found: $Path"
  }
}



function Add-ReviewLine {
  param(
    [string]$Text
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "$Text`r`n"
}




function Get-AggregateComparisonSkipReason {
  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $PreviousTrialSummaryCsv `
    -CurrentPath $CurrentTrialSummaryCsv `
    -PreviousDescription "previous aggregate trial summary" `
    -CurrentDescription "current aggregate trial summary"
}

function Get-WorkloadComparisonSkipReason {
  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $PreviousWorkloadSummaryCsv `
    -CurrentPath $CurrentWorkloadSummaryCsv `
    -PreviousDescription "previous workload trial summary" `
    -CurrentDescription "current workload trial summary"
}

function Get-CountOnlyComparisonSkipReason {
  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $PreviousCountOnlySummaryCsv `
    -CurrentPath $CurrentCountOnlyWorkloadSummaryCsv `
    -PreviousDescription "previous count-only workload summary" `
    -CurrentDescription "current count-only workload summary"
}

function Get-TargetComparisonSkipReason {
  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $PreviousTargetSummaryCsv `
    -CurrentPath $CurrentTargetSummaryCsv `
    -PreviousDescription "previous target workload summary" `
    -CurrentDescription "current target workload summary" `
    -Disabled ([bool]$SkipTargetWorkloadReview) `
    -DisabledReason "target workload review was disabled"
}

function Write-ReviewManifest {
  $AggregateComparisonSkipReason = Get-AggregateComparisonSkipReason
  $WorkloadComparisonSkipReason = Get-WorkloadComparisonSkipReason
  $CountOnlyComparisonSkipReason = Get-CountOnlyComparisonSkipReason
  $TargetComparisonSkipReason = Get-TargetComparisonSkipReason

  $AggregateComparisonSkipped = ![string]::IsNullOrWhiteSpace($AggregateComparisonSkipReason)
  $WorkloadComparisonSkipped = ![string]::IsNullOrWhiteSpace($WorkloadComparisonSkipReason)
  $CountOnlyComparisonSkipped = ![string]::IsNullOrWhiteSpace($CountOnlyComparisonSkipReason)
  $TargetSummarySkipped = [bool]$SkipTargetWorkloadReview
  $TargetComparisonSkipped = ![string]::IsNullOrWhiteSpace($TargetComparisonSkipReason)

  Write-BenchmarkReviewManifestHeader `
    -ManifestPath $ReviewManifestPath `
    -Label $Label `
    -PreviousLabel $PreviousLabel `
    -Dataset $Dataset `
    -OrganizedRunRoot $OrganizedRunRoot `
    -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory `
    -MaxDepth $EffectiveMaxDepth `
    -Trials $Trials `
    -Iterations $Iterations `
    -TargetWorkloadName $TargetWorkloadName `
    -CopyArtifactsToRunFolder $CopyArtifactsToRunFolder `
    -ForceOrganizedArtifacts $ForceOrganizedArtifacts `
    -ValidateArtifacts $ValidateArtifacts `
    -RequireValidatedComparisons $RequireValidatedComparisons `
    -CleanupFlatArtifacts $CleanupFlatArtifacts `
    -NoiseThresholdText (Format-InvariantDouble -Value $NoiseThreshold)

  $TargetSummarySkipReason = if ($TargetSummarySkipped) { "target workload review was disabled" } else { "" }

  $ManifestArtifacts = @(
    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.BenchmarkOutput `
      -Path (Join-Path $OutputDir "benchmark-output-$Label.txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.DebugOutput `
      -Path (Join-Path $OutputDir "debug-output-$Label.txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.SummaryCsv `
      -Path (Join-Path $OutputDir "summary-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.WorkloadsCsv `
      -Path (Join-Path $OutputDir "workloads-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadSummary `
      -Path (Join-Path $OutputDir "count-only-workload-summary-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadNotes `
      -Path (Join-Path $OutputDir "count-only-workload-summary-$Label.txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonCsv `
      -Path (Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.csv") `
      -Skipped $CountOnlyComparisonSkipped `
      -Note $CountOnlyComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonNotes `
      -Path (Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.txt") `
      -Skipped $CountOnlyComparisonSkipped `
      -Note $CountOnlyComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowSelectivityGapCsv `
      -Path (Join-Path $OutputDir "low-selectivity-gap-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapRegressionNotes `
      -Path (Join-Path $OutputDir "low-gap-regression-notes-$Label.txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapTrialDetails `
      -Path (Join-Path $OutputDir "low-gap-trial-details-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapTrialSummary `
      -Path (Join-Path $OutputDir "low-gap-trial-summary-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapTrialNotes `
      -Path (Join-Path $OutputDir "low-gap-trial-notes-$Label.txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapWorkloadTrialDetails `
      -Path (Join-Path $OutputDir "low-gap-workload-trial-details-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapWorkloadTrialSummary `
      -Path (Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.AggregateTrialComparisonCsv `
      -Path (Join-Path $OutputDir "low-gap-trial-comparison-$Label.csv") `
      -Skipped $AggregateComparisonSkipped `
      -Note $AggregateComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.AggregateTrialComparisonNotes `
      -Path (Join-Path $OutputDir "low-gap-trial-comparison-$Label.txt") `
      -Skipped $AggregateComparisonSkipped `
      -Note $AggregateComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.WorkloadTrialComparisonCsv `
      -Path (Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.csv") `
      -Skipped $WorkloadComparisonSkipped `
      -Note $WorkloadComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.WorkloadTrialComparisonNotes `
      -Path (Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.txt") `
      -Skipped $WorkloadComparisonSkipped `
      -Note $WorkloadComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialDetails `
      -Path (Join-Path $OutputDir "target-workload-trial-details-$Label.csv") `
      -Skipped $TargetSummarySkipped `
      -Note $TargetSummarySkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialSummary `
      -Path (Join-Path $OutputDir "target-workload-trial-summary-$Label.csv") `
      -Skipped $TargetSummarySkipped `
      -Note $TargetSummarySkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialNotes `
      -Path (Join-Path $OutputDir "target-workload-trial-notes-$Label.txt") `
      -Skipped $TargetSummarySkipped `
      -Note $TargetSummarySkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonCsv `
      -Path (Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv") `
      -Skipped $TargetComparisonSkipped `
      -Note $TargetComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonNotes `
      -Path (Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt") `
      -Skipped $TargetComparisonSkipped `
      -Note $TargetComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapReviewNotes `
      -Path $ReviewNotesPath `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapReviewManifest `
      -Path $ReviewManifestPath `
      -Skipped $false
  )

  Add-BenchmarkReviewManifestArtifacts `
    -ManifestPath $ReviewManifestPath `
    -Artifacts $ManifestArtifacts

  Add-BenchmarkReviewManifestInterpretation -ManifestPath $ReviewManifestPath
}

Require-Script -Path $SmallBenchmarkScript
Require-Script -Path $LowGapTrialsScript
Require-Script -Path $AggregateComparatorScript
Require-Script -Path $WorkloadComparatorScript
Require-Script -Path $CountOnlyComparatorScript

if (!$SkipTargetWorkloadReview) {
  Require-Script -Path $TargetSummaryScript
  Require-Script -Path $TargetComparatorScript
}

if ($CopyArtifactsToRunFolder) {
  Require-Script -Path $ArtifactOrganizerScript
}

if ($ValidateArtifacts) {
  Require-Script -Path $ArtifactValidatorScript
}

if ($CleanupFlatArtifacts) {
  Require-Script -Path $FlatArtifactCleanupScript
}

$PreviousLowSelectivityGapCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousLowSelectivityGapCsv `
  -DefaultPath (Join-Path $OutputDir "low-selectivity-gap-$PreviousLabel.csv") `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousTrialSummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousTrialSummaryCsv `
  -DefaultPath (Join-Path $OutputDir "low-gap-trial-summary-$PreviousLabel.csv") `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousWorkloadSummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousWorkloadSummaryCsv `
  -DefaultPath (Join-Path $OutputDir "low-gap-workload-trial-summary-$PreviousLabel.csv") `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousCountOnlySummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousCountOnlySummaryCsv `
  -DefaultPath (Join-Path $OutputDir "count-only-workload-summary-$PreviousLabel.csv") `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory
  
$PreviousTargetSummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousTargetSummaryCsv `
  -DefaultPath (Join-Path $OutputDir "target-workload-trial-summary-$PreviousLabel.csv") `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousLowSelectivityGapInput = Get-BenchmarkReviewInputStatus -Path $PreviousLowSelectivityGapCsv
$PreviousTrialSummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousTrialSummaryCsv
$PreviousWorkloadSummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousWorkloadSummaryCsv
$PreviousCountOnlySummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousCountOnlySummaryCsv
$PreviousTargetSummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousTargetSummaryCsv

Set-Utf8Text -Path $ReviewNotesPath -Text "Low-gap benchmark review`r`n"
Add-ReviewLine "========================"
Add-ReviewLine ""
Add-ReviewLine "Label: $Label"
Add-ReviewLine "Previous label: $PreviousLabel"
Add-ReviewLine "Dataset: $Dataset"
Add-ReviewLine "Max depth: $EffectiveMaxDepth"
Add-ReviewLine "Trials: $Trials"
Add-ReviewLine "Iterations: $Iterations"
Add-ReviewLine "Output directory: $OutputDir"
Add-ReviewLine "Organized run root: $OrganizedRunRoot"
Add-ReviewLine "Previous organized run directory: $PreviousOrganizedRunDirectory"
Add-ReviewLine "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)"
Add-ReviewLine "Target workload review skipped: $SkipTargetWorkloadReview"
Add-ReviewLine "Target workload name: $TargetWorkloadName"
Add-ReviewLine "Copy artifacts to run folder: $CopyArtifactsToRunFolder"
Add-ReviewLine "Force organized artifacts: $ForceOrganizedArtifacts"
Add-ReviewLine "Validate artifacts: $ValidateArtifacts"
Add-ReviewLine "Require validated comparisons: $RequireValidatedComparisons"
Add-ReviewLine "Cleanup flat artifacts: $CleanupFlatArtifacts"
Add-ReviewLine "Previous low-selectivity gap CSV: $PreviousLowSelectivityGapCsv"
Add-ReviewLine "Previous aggregate trial summary CSV: $PreviousTrialSummaryCsv"
Add-ReviewLine "Previous workload trial summary CSV: $PreviousWorkloadSummaryCsv"
Add-ReviewLine "Previous count-only workload summary CSV: $PreviousCountOnlySummaryCsv"
Add-ReviewLine "Previous target workload summary CSV: $PreviousTargetSummaryCsv"
Add-ReviewLine ""

Add-BenchmarkReviewPreviousInputStatusLines `
  -ReviewNotesPath $ReviewNotesPath `
  -SingleRunInput $PreviousLowSelectivityGapInput `
  -AggregateInput $PreviousTrialSummaryInput `
  -WorkloadInput $PreviousWorkloadSummaryInput `
  -TargetInput $PreviousTargetSummaryInput

Add-ReviewLine "count-only workload summary status: $($PreviousCountOnlySummaryInput.Status)"

if ($PreviousCountOnlySummaryInput.Exists) {
  Add-ReviewLine "count-only workload summary resolved path: $($PreviousCountOnlySummaryInput.ResolvedPath)"
}

Add-ReviewLine ""

Write-Host ""
Write-Host "Running normal benchmark artifact bundle"
Write-Host "  label: $Label"
Write-Host "  dataset: $Dataset"
Write-Host "  max depth: $EffectiveMaxDepth"

Add-ReviewLine "Normal benchmark artifact bundle"
Add-ReviewLine "-------------------------------"
Add-ReviewLine "status: running"

if ($PreviousLowSelectivityGapInput.Exists) {
  Add-ReviewLine "single-run comparison input: found"
}
elseif ([string]::IsNullOrWhiteSpace($PreviousLowSelectivityGapCsv)) {
  Add-ReviewLine "single-run comparison input: not provided"
}
else {
  Add-ReviewLine "single-run comparison input: path not found"
}

& $SmallBenchmarkScript `
  -Label $Label `
  -Dataset $Dataset `
  -MaxDepth $EffectiveMaxDepth `
  -Iterations $Iterations `
  -OutputDir $OutputDir `
  -PreviousLowSelectivityGapCsv $PreviousLowSelectivityGapCsv `
  -LowGapNoiseThreshold $NoiseThreshold

if ($LASTEXITCODE -ne 0) {
  throw "normal benchmark artifact bundle failed with exit code $LASTEXITCODE"
}

Add-ReviewLine "status: completed"
Add-ReviewLine "count-only workload summary: $CurrentCountOnlyWorkloadSummaryCsv"
Add-ReviewLine "count-only workload notes: $CurrentCountOnlyWorkloadSummaryNotes"

if ($PreviousLowSelectivityGapInput.Exists) {
  Add-ReviewLine "single-run comparison status: completed by run-small-benchmark"
}
else {
  Add-ReviewLine "single-run comparison status: unavailable"
  Add-ReviewLine "single-run comparison reason: previous low-selectivity gap CSV was $($PreviousLowSelectivityGapInput.Status)"
}

Add-ReviewLine ""

Write-Host ""
Write-Host "Running repeated low-gap trials"
Write-Host "  label: $Label"
Write-Host "  dataset: $Dataset"
Write-Host "  trials: $Trials"

Add-ReviewLine "Repeated low-gap trials"
Add-ReviewLine "-----------------------"
Add-ReviewLine "status: running"

& $LowGapTrialsScript `
  -Label $Label `
  -Dataset $Dataset `
  -MaxDepth $EffectiveMaxDepth `
  -Trials $Trials `
  -Iterations $Iterations `
  -OutputDir $OutputDir `
  -PreviousLowSelectivityGapCsv $PreviousLowSelectivityGapCsv `
  -LowGapNoiseThreshold $NoiseThreshold

if ($LASTEXITCODE -ne 0) {
  throw "repeated low-gap trials failed with exit code $LASTEXITCODE"
}

Add-ReviewLine "status: completed"
Add-ReviewLine "current aggregate trial summary: $CurrentTrialSummaryCsv"
Add-ReviewLine "current workload trial summary: $CurrentWorkloadSummaryCsv"

if ($PreviousLowSelectivityGapInput.Exists) {
  Add-ReviewLine "trial notes previous single-run input: found"
}
else {
  Add-ReviewLine "trial notes previous single-run input: $($PreviousLowSelectivityGapInput.Status)"
}

Add-ReviewLine ""

Write-Host ""
Write-Host "Running target workload trial summary"
Write-Host "  dataset: $Dataset"
Write-Host "  max depth: $EffectiveMaxDepth"
Write-Host "  target workload: $TargetWorkloadName"

Add-ReviewLine "Target workload trial summary"
Add-ReviewLine "-----------------------------"

if ($SkipTargetWorkloadReview) {
  Write-Host "  skipped: target workload review was disabled"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: -SkipTargetWorkloadReview was passed"
}
else {
  Add-ReviewLine "status: running"
  Add-ReviewLine "target workload: $TargetWorkloadName"

  & $TargetSummaryScript `
    -Label $Label `
    -InputLabel $Label `
    -TargetWorkloadName $TargetWorkloadName `
    -OutputDir $OutputDir

  if ($LASTEXITCODE -ne 0) {
    throw "target workload trial summary failed with exit code $LASTEXITCODE"
  }

  Add-ReviewLine "status: completed"
  Add-ReviewLine "details CSV: $CurrentTargetDetailCsv"
  Add-ReviewLine "summary CSV: $CurrentTargetSummaryCsv"
  Add-ReviewLine "notes: $CurrentTargetNotesPath"
}

Add-ReviewLine ""

$TargetComparatorArguments = @{
  Label                    = $Label
  PreviousTargetSummaryCsv = $PreviousTargetSummaryCsv
  CurrentTargetSummaryCsv  = $CurrentTargetSummaryCsv
  OutputDir                = $OutputDir
  NoiseThreshold           = $NoiseThreshold
}

Invoke-BenchmarkReviewComparisonStep `
  -ReviewNotesPath $ReviewNotesPath `
  -HostTitle "target workload trial comparison" `
  -ReviewTitle "Target workload trial comparison" `
  -ReviewUnderline "--------------------------------" `
  -PreviousPath $PreviousTargetSummaryCsv `
  -CurrentPath $CurrentTargetSummaryCsv `
  -PreviousDescription "previous target workload summary" `
  -CurrentDescription "current target workload summary" `
  -ScriptPath $TargetComparatorScript `
  -ScriptArguments $TargetComparatorArguments `
  -FailureMessage "target workload trial comparison failed with exit code" `
  -CompletedLines @(
  "comparison CSV: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv")",
  "comparison notes: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt")"
) `
  -Disabled ([bool]$SkipTargetWorkloadReview) `
  -DisabledConsoleReason "target workload review was disabled" `
  -DisabledReviewReason "-SkipTargetWorkloadReview was passed"

$CountOnlyComparatorArguments = @{
  Label                       = $Label
  PreviousCountOnlySummaryCsv = $PreviousCountOnlySummaryCsv
  CurrentCountOnlySummaryCsv  = $CurrentCountOnlyWorkloadSummaryCsv
  OutputDir                   = $OutputDir
  NoiseThreshold              = $NoiseThreshold
}

Invoke-BenchmarkReviewComparisonStep `
  -ReviewNotesPath $ReviewNotesPath `
  -HostTitle "count-only workload summary comparison" `
  -ReviewTitle "Count-only workload summary comparison" `
  -ReviewUnderline "--------------------------------------" `
  -PreviousPath $PreviousCountOnlySummaryCsv `
  -CurrentPath $CurrentCountOnlyWorkloadSummaryCsv `
  -PreviousDescription "previous count-only workload summary" `
  -CurrentDescription "current count-only workload summary" `
  -ScriptPath $CountOnlyComparatorScript `
  -ScriptArguments $CountOnlyComparatorArguments `
  -FailureMessage "count-only workload summary comparison failed with exit code" `
  -CompletedLines @(
  "comparison CSV: $CurrentCountOnlyComparisonCsv",
  "comparison notes: $CurrentCountOnlyComparisonNotes"
)

$AggregateComparatorArguments = @{
  Label                   = $Label
  PreviousTrialSummaryCsv = $PreviousTrialSummaryCsv
  CurrentTrialSummaryCsv  = $CurrentTrialSummaryCsv
  OutputDir               = $OutputDir
  NoiseThreshold          = $NoiseThreshold
}

Invoke-BenchmarkReviewComparisonStep `
  -ReviewNotesPath $ReviewNotesPath `
  -HostTitle "aggregate trial comparison" `
  -ReviewTitle "Aggregate trial comparison" `
  -ReviewUnderline "--------------------------" `
  -PreviousPath $PreviousTrialSummaryCsv `
  -CurrentPath $CurrentTrialSummaryCsv `
  -PreviousDescription "previous aggregate trial summary" `
  -CurrentDescription "current aggregate trial summary" `
  -ScriptPath $AggregateComparatorScript `
  -ScriptArguments $AggregateComparatorArguments `
  -FailureMessage "aggregate trial comparison failed with exit code" `
  -CompletedLines @(
  "comparison CSV: $(Join-Path $OutputDir "low-gap-trial-comparison-$Label.csv")",
  "comparison notes: $(Join-Path $OutputDir "low-gap-trial-comparison-$Label.txt")"
)

$WorkloadComparatorArguments = @{
  Label                      = $Label
  PreviousWorkloadSummaryCsv = $PreviousWorkloadSummaryCsv
  CurrentWorkloadSummaryCsv  = $CurrentWorkloadSummaryCsv
  OutputDir                  = $OutputDir
  NoiseThreshold             = $NoiseThreshold
}

Invoke-BenchmarkReviewComparisonStep `
  -ReviewNotesPath $ReviewNotesPath `
  -HostTitle "workload trial comparison" `
  -ReviewTitle "Workload trial comparison" `
  -ReviewUnderline "-------------------------" `
  -PreviousPath $PreviousWorkloadSummaryCsv `
  -CurrentPath $CurrentWorkloadSummaryCsv `
  -PreviousDescription "previous workload trial summary" `
  -CurrentDescription "current workload trial summary" `
  -ScriptPath $WorkloadComparatorScript `
  -ScriptArguments $WorkloadComparatorArguments `
  -FailureMessage "workload trial comparison failed with exit code" `
  -CompletedLines @(
  "comparison CSV: $(Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.csv")",
  "comparison notes: $(Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.txt")"
)

Add-BenchmarkReviewComparisonAvailabilitySummary `
  -ReviewNotesPath $ReviewNotesPath `
  -PreviousLowSelectivityGapInput $PreviousLowSelectivityGapInput `
  -PreviousTrialSummaryInput $PreviousTrialSummaryInput `
  -CurrentTrialSummaryCsv $CurrentTrialSummaryCsv `
  -PreviousWorkloadSummaryInput $PreviousWorkloadSummaryInput `
  -CurrentWorkloadSummaryCsv $CurrentWorkloadSummaryCsv `
  -PreviousCountOnlySummaryInput $PreviousCountOnlySummaryInput `
  -CurrentCountOnlyWorkloadSummaryCsv $CurrentCountOnlyWorkloadSummaryCsv `
  -SkipTargetWorkloadReview ([bool]$SkipTargetWorkloadReview) `
  -PreviousTargetSummaryInput $PreviousTargetSummaryInput `
  -CurrentTargetSummaryCsv $CurrentTargetSummaryCsv

Add-BenchmarkReviewArtifactSummary `
  -ReviewNotesPath $ReviewNotesPath `
  -OutputDir $OutputDir `
  -Label $Label `
  -ReviewManifestPath $ReviewManifestPath `
  -TargetWorkloadName $TargetWorkloadName

Write-ReviewManifest

if ($CopyArtifactsToRunFolder) {
  Invoke-BenchmarkReviewOrganizedArtifactCopy `
    -ReviewNotesPath $ReviewNotesPath `
    -OutputDir $OutputDir `
    -Label $Label `
    -OrganizedRunDirectory $OrganizedRunDirectory `
    -ForceOrganizedArtifacts ([bool]$ForceOrganizedArtifacts) `
    -ArtifactOrganizerScript $ArtifactOrganizerScript
}

if ($ValidateArtifacts) {
  Add-ReviewLine ""
  Add-ReviewLine "Review artifact validation"
  Add-ReviewLine "--------------------------"
  Add-ReviewLine "status: running"

  Write-Host ""
  Write-Host "Validating review artifacts"
  Write-Host "  label: $Label"

  $ValidatorArguments = @{
    Label          = $Label
    OutputDir      = $OutputDir
    ExpectedTrials = $Trials
  }

  if ($RequireValidatedComparisons) {
    $ValidatorArguments["RequireComparisons"] = $true
  }

  if ($SkipTargetWorkloadReview) {
    $ValidatorArguments["AllowSkippedTargetWorkloadReview"] = $true
  }

  & $ArtifactValidatorScript @ValidatorArguments

  if ($LASTEXITCODE -ne 0) {
    Add-ReviewLine "status: failed"
    throw "review artifact validation failed with exit code $LASTEXITCODE"
  }

  Add-ReviewLine "status: completed"
}

Copy-BenchmarkReviewNotesToOrganizedRun `
  -ReviewNotesPath $ReviewNotesPath `
  -OrganizedRunDirectory $OrganizedRunDirectory `
  -CopyArtifactsToRunFolder ([bool]$CopyArtifactsToRunFolder)

if ($CleanupFlatArtifacts) {
  Add-ReviewLine ""
  Add-ReviewLine "Flat artifact cleanup"
  Add-ReviewLine "---------------------"
  Add-ReviewLine "status: requested"
  Add-ReviewLine "reason: organized artifacts were copied and validated before flat cleanup"

  Copy-BenchmarkReviewNotesToOrganizedRun `
    -ReviewNotesPath $ReviewNotesPath `
    -OrganizedRunDirectory $OrganizedRunDirectory `
    -CopyArtifactsToRunFolder ([bool]$CopyArtifactsToRunFolder)

  Write-Host ""
  Write-Host "Cleaning flat benchmark artifacts"
  Write-Host "  label: $Label"
  Write-Host "  artifact root: $OutputDir"

  & $FlatArtifactCleanupScript `
    -ArtifactRoot $OutputDir `
    -Label $Label `
    -Delete `
    -Force

  if ($LASTEXITCODE -ne 0) {
    throw "flat artifact cleanup failed with exit code $LASTEXITCODE"
  }
}

Write-BenchmarkReviewCompletionOutput `
  -ReviewNotesPath $ReviewNotesPath `
  -ReviewManifestPath $ReviewManifestPath `
  -OrganizedRunDirectory $OrganizedRunDirectory `
  -CopyArtifactsToRunFolder ([bool]$CopyArtifactsToRunFolder) `
  -CleanupFlatArtifacts ([bool]$CleanupFlatArtifacts)
