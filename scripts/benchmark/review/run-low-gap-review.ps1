param(
  [string]$Label = "",
  [ValidateRange(0, 2147483647)]
  [int]$RunNumber = 0,
  [ValidateRange(1, 2147483647)]
  [int]$RunIndex = 1,
  [string]$RunTopic = "",
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
  [string]$PreviousMaterializationSummaryCsv = "",
  [string]$TargetWorkloadName = "",
  [string]$PreviousTargetSummaryCsv = "",
  [switch]$SkipTargetWorkloadReview,
  [switch]$CopyArtifactsToRunFolder,
  [switch]$ForceOrganizedArtifacts,
  [switch]$ValidateArtifacts,
  [switch]$RequireValidatedComparisons,
  [switch]$CleanupFlatArtifacts,
  [switch]$UpdateHistory,
  [string]$HistoryDir = "",
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$BenchmarkCsvLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-csv.ps1"
$BenchmarkReviewManifestLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-review-manifest.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkReviewManifestLibrary)) {
  throw "benchmark review manifest helper library was not found: $BenchmarkReviewManifestLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkReviewManifestLibrary

$Label = Resolve-BenchmarkReviewLabel `
  -Label $Label `
  -RunNumber $RunNumber `
  -RunIndex $RunIndex `
  -RunTopic $RunTopic `
  -Dataset $Dataset

$RunId = ""
$AttemptId = ""
$NormalizedRunTopic = ""

if ($RunNumber -gt 0) {
  $RunId = Format-BenchmarkReviewRunId -RunNumber $RunNumber
  $AttemptId = Format-BenchmarkReviewAttemptId -RunIndex $RunIndex
  $NormalizedRunTopic = Normalize-BenchmarkReviewRunTopic -RunTopic $RunTopic
}

$TargetWorkloadName = Resolve-BenchmarkReviewTargetWorkloadName `
  -TargetWorkloadName $TargetWorkloadName `
  -Dataset $Dataset

$EffectiveMaxDepth = Resolve-BenchmarkReviewEffectiveMaxDepth `
  -MaxDepth $MaxDepth `
  -Dataset $Dataset

Assert-BenchmarkReviewPreflight `
  -Label $Label `
  -Trials $Trials `
  -TargetWorkloadName $TargetWorkloadName `
  -CleanupFlatArtifacts ([bool]$CleanupFlatArtifacts) `
  -CopyArtifactsToRunFolder ([bool]$CopyArtifactsToRunFolder) `
  -UpdateHistory ([bool]$UpdateHistory) `
  -ValidateArtifacts ([bool]$ValidateArtifacts) `
  -RunNumber $RunNumber `
  -RunTopic $RunTopic

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ([string]::IsNullOrWhiteSpace($HistoryDir)) {
  $HistoryDir = Join-Path $OutputDir "history"
}

$ReviewScriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$BenchmarkScriptRoot = Split-Path -Parent $ReviewScriptDirectory

$ReviewPaths = New-BenchmarkReviewPathSet `
  -ScriptDirectory $BenchmarkScriptRoot `
  -OutputDir $OutputDir `
  -Label $Label `
  -PreviousLabel $PreviousLabel

$SmallBenchmarkScript = $ReviewPaths.SmallBenchmarkScript
$LowGapTrialsScript = $ReviewPaths.LowGapTrialsScript
$AggregateComparatorScript = $ReviewPaths.AggregateComparatorScript
$WorkloadComparatorScript = $ReviewPaths.WorkloadComparatorScript
$CountOnlyComparatorScript = $ReviewPaths.CountOnlyComparatorScript
$MaterializationComparatorScript = $ReviewPaths.MaterializationComparatorScript
$TargetSummaryScript = $ReviewPaths.TargetSummaryScript
$TargetComparatorScript = $ReviewPaths.TargetComparatorScript
$ArtifactOrganizerScript = $ReviewPaths.ArtifactOrganizerScript
$ArtifactValidatorScript = $ReviewPaths.ArtifactValidatorScript
$FlatArtifactCleanupScript = $ReviewPaths.FlatArtifactCleanupScript
$OrganizedRunRoot = $ReviewPaths.OrganizedRunRoot
$OrganizedRunDirectory = $ReviewPaths.OrganizedRunDirectory
$PreviousOrganizedRunDirectory = $ReviewPaths.PreviousOrganizedRunDirectory
$ReviewNotesPath = $ReviewPaths.ReviewNotesPath
$ReviewManifestPath = $ReviewPaths.ReviewManifestPath
$CurrentTrialSummaryCsv = $ReviewPaths.CurrentTrialSummaryCsv
$CurrentWorkloadSummaryCsv = $ReviewPaths.CurrentWorkloadSummaryCsv
$CurrentCountOnlyWorkloadSummaryCsv = $ReviewPaths.CurrentCountOnlyWorkloadSummaryCsv
$CurrentCountOnlyWorkloadSummaryNotes = $ReviewPaths.CurrentCountOnlyWorkloadSummaryNotes
$CurrentCountOnlyComparisonCsv = $ReviewPaths.CurrentCountOnlyComparisonCsv
$CurrentCountOnlyComparisonNotes = $ReviewPaths.CurrentCountOnlyComparisonNotes
$CurrentMaterializationSummaryCsv = $ReviewPaths.CurrentMaterializationSummaryCsv
$CurrentMaterializationSummaryNotes = $ReviewPaths.CurrentMaterializationSummaryNotes
$CurrentTypedArchiveLoadSummaryCsv = $ReviewPaths.CurrentTypedArchiveLoadSummaryCsv
$CurrentTypedArchiveLoadSummaryNotes = $ReviewPaths.CurrentTypedArchiveLoadSummaryNotes
$CurrentMaterializationComparisonCsv = $ReviewPaths.CurrentMaterializationComparisonCsv
$CurrentMaterializationComparisonNotes = $ReviewPaths.CurrentMaterializationComparisonNotes
$CurrentTargetDetailCsv = $ReviewPaths.CurrentTargetDetailCsv
$CurrentTargetSummaryCsv = $ReviewPaths.CurrentTargetSummaryCsv
$CurrentTargetNotesPath = $ReviewPaths.CurrentTargetNotesPath

function Add-ReviewLine {
  param(
    [string]$Text
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "$Text`r`n"
}





$RequiredReviewScripts = Get-BenchmarkReviewRequiredScripts `
  -PathSet $ReviewPaths `
  -SkipTargetWorkloadReview ([bool]$SkipTargetWorkloadReview) `
  -CopyArtifactsToRunFolder ([bool]$CopyArtifactsToRunFolder) `
  -ValidateArtifacts ([bool]$ValidateArtifacts) `
  -CleanupFlatArtifacts ([bool]$CleanupFlatArtifacts)

Assert-BenchmarkReviewScripts -Paths $RequiredReviewScripts

$PreviousLowSelectivityGapCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousLowSelectivityGapCsv `
  -DefaultPath $ReviewPaths.PreviousLowSelectivityGapDefaultPath `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousTrialSummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousTrialSummaryCsv `
  -DefaultPath $ReviewPaths.PreviousTrialSummaryDefaultPath `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousWorkloadSummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousWorkloadSummaryCsv `
  -DefaultPath $ReviewPaths.PreviousWorkloadSummaryDefaultPath `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousCountOnlySummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousCountOnlySummaryCsv `
  -DefaultPath $ReviewPaths.PreviousCountOnlySummaryDefaultPath `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousMaterializationSummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousMaterializationSummaryCsv `
  -DefaultPath $ReviewPaths.PreviousMaterializationSummaryDefaultPath `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory
  
$PreviousTargetSummaryCsv = Resolve-BenchmarkReviewOptionalPreviousPath `
  -ExplicitPath $PreviousTargetSummaryCsv `
  -DefaultPath $ReviewPaths.PreviousTargetSummaryDefaultPath `
  -PreviousLabel $PreviousLabel `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory

$PreviousLowSelectivityGapInput = Get-BenchmarkReviewInputStatus -Path $PreviousLowSelectivityGapCsv
$PreviousTrialSummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousTrialSummaryCsv
$PreviousWorkloadSummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousWorkloadSummaryCsv
$PreviousCountOnlySummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousCountOnlySummaryCsv
$PreviousMaterializationSummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousMaterializationSummaryCsv
$PreviousTargetSummaryInput = Get-BenchmarkReviewInputStatus -Path $PreviousTargetSummaryCsv

Initialize-BenchmarkReviewNotes `
  -ReviewNotesPath $ReviewNotesPath `
  -Label $Label `
  -RunId $RunId `
  -AttemptId $AttemptId `
  -RunTopic $NormalizedRunTopic `
  -PreviousLabel $PreviousLabel `
  -Dataset $Dataset `
  -MaxDepth $EffectiveMaxDepth `
  -Trials $Trials `
  -Iterations $Iterations `
  -OutputDir $OutputDir `
  -OrganizedRunRoot $OrganizedRunRoot `
  -PreviousOrganizedRunDirectory $PreviousOrganizedRunDirectory `
  -NoiseThreshold $NoiseThreshold `
  -SkipTargetWorkloadReview ([bool]$SkipTargetWorkloadReview) `
  -TargetWorkloadName $TargetWorkloadName `
  -CopyArtifactsToRunFolder ([bool]$CopyArtifactsToRunFolder) `
  -ForceOrganizedArtifacts ([bool]$ForceOrganizedArtifacts) `
  -ValidateArtifacts ([bool]$ValidateArtifacts) `
  -RequireValidatedComparisons ([bool]$RequireValidatedComparisons) `
  -CleanupFlatArtifacts ([bool]$CleanupFlatArtifacts) `
  -UpdateHistory ([bool]$UpdateHistory) `
  -HistoryDir $HistoryDir `
  -PreviousLowSelectivityGapCsv $PreviousLowSelectivityGapCsv `
  -PreviousTrialSummaryCsv $PreviousTrialSummaryCsv `
  -PreviousWorkloadSummaryCsv $PreviousWorkloadSummaryCsv `
  -PreviousCountOnlySummaryCsv $PreviousCountOnlySummaryCsv `
  -PreviousMaterializationSummaryCsv $PreviousMaterializationSummaryCsv `
  -PreviousTargetSummaryCsv $PreviousTargetSummaryCsv `
  -PreviousLowSelectivityGapInput $PreviousLowSelectivityGapInput `
  -PreviousTrialSummaryInput $PreviousTrialSummaryInput `
  -PreviousWorkloadSummaryInput $PreviousWorkloadSummaryInput `
  -PreviousCountOnlySummaryInput $PreviousCountOnlySummaryInput `
  -PreviousMaterializationSummaryInput $PreviousMaterializationSummaryInput `
  -PreviousTargetSummaryInput $PreviousTargetSummaryInput

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
Add-ReviewLine "materialization mode summary: $CurrentMaterializationSummaryCsv"
Add-ReviewLine "materialization mode notes: $CurrentMaterializationSummaryNotes"
Add-ReviewLine "typed archive load summary: $CurrentTypedArchiveLoadSummaryCsv"
Add-ReviewLine "typed archive load notes: $CurrentTypedArchiveLoadSummaryNotes"

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

$MaterializationComparatorArguments = @{
  Label                             = $Label
  PreviousMaterializationSummaryCsv = $PreviousMaterializationSummaryCsv
  CurrentMaterializationSummaryCsv  = $CurrentMaterializationSummaryCsv
  OutputDir                         = $OutputDir
  NoiseThreshold                    = $NoiseThreshold
}

Invoke-BenchmarkReviewComparisonStep `
  -ReviewNotesPath $ReviewNotesPath `
  -HostTitle "materialization mode summary comparison" `
  -ReviewTitle "Materialization mode summary comparison" `
  -ReviewUnderline "---------------------------------------" `
  -PreviousPath $PreviousMaterializationSummaryCsv `
  -CurrentPath $CurrentMaterializationSummaryCsv `
  -PreviousDescription "previous materialization mode summary" `
  -CurrentDescription "current materialization mode summary" `
  -ScriptPath $MaterializationComparatorScript `
  -ScriptArguments $MaterializationComparatorArguments `
  -FailureMessage "materialization mode summary comparison failed with exit code" `
  -CompletedLines @(
  "comparison CSV: $CurrentMaterializationComparisonCsv",
  "comparison notes: $CurrentMaterializationComparisonNotes"
)

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
  -PreviousMaterializationSummaryInput $PreviousMaterializationSummaryInput `
  -CurrentMaterializationSummaryCsv $CurrentMaterializationSummaryCsv `
  -SkipTargetWorkloadReview ([bool]$SkipTargetWorkloadReview) `
  -PreviousTargetSummaryInput $PreviousTargetSummaryInput `
  -CurrentTargetSummaryCsv $CurrentTargetSummaryCsv

Add-BenchmarkReviewArtifactSummary `
  -ReviewNotesPath $ReviewNotesPath `
  -OutputDir $OutputDir `
  -Label $Label `
  -ReviewManifestPath $ReviewManifestPath `
  -TargetWorkloadName $TargetWorkloadName

$ReviewManifestContext = [PSCustomObject]@{
  ReviewManifestPath                    = $ReviewManifestPath
  ReviewNotesPath                       = $ReviewNotesPath
  Label                                 = $Label
  RunId                                 = $RunId
  AttemptId                             = $AttemptId
  RunTopic                              = $NormalizedRunTopic
  PreviousLabel                         = $PreviousLabel
  Dataset                               = $Dataset
  OutputDir                             = $OutputDir
  OrganizedRunRoot                      = $OrganizedRunRoot
  OrganizedRunDirectory                 = $OrganizedRunDirectory
  PreviousOrganizedRunDirectory         = $PreviousOrganizedRunDirectory
  EffectiveMaxDepth                     = $EffectiveMaxDepth
  Trials                                = $Trials
  Iterations                            = $Iterations
  TargetWorkloadName                    = $TargetWorkloadName
  CopyArtifactsToRunFolder              = [bool]$CopyArtifactsToRunFolder
  ForceOrganizedArtifacts               = [bool]$ForceOrganizedArtifacts
  ValidateArtifacts                     = [bool]$ValidateArtifacts
  RequireValidatedComparisons           = [bool]$RequireValidatedComparisons
  CleanupFlatArtifacts                  = [bool]$CleanupFlatArtifacts
  UpdateHistory                         = [bool]$UpdateHistory
  HistoryDir                            = $HistoryDir
  SkipTargetWorkloadReview              = [bool]$SkipTargetWorkloadReview
  NoiseThreshold                        = $NoiseThreshold
  PreviousTrialSummaryCsv               = $PreviousTrialSummaryCsv
  CurrentTrialSummaryCsv                = $CurrentTrialSummaryCsv
  PreviousWorkloadSummaryCsv            = $PreviousWorkloadSummaryCsv
  CurrentWorkloadSummaryCsv             = $CurrentWorkloadSummaryCsv
  PreviousCountOnlySummaryCsv           = $PreviousCountOnlySummaryCsv
  CurrentCountOnlyWorkloadSummaryCsv    = $CurrentCountOnlyWorkloadSummaryCsv
  PreviousMaterializationSummaryCsv     = $PreviousMaterializationSummaryCsv
  CurrentMaterializationSummaryCsv      = $CurrentMaterializationSummaryCsv
  CurrentMaterializationSummaryNotes    = $CurrentMaterializationSummaryNotes
  CurrentTypedArchiveLoadSummaryCsv     = $CurrentTypedArchiveLoadSummaryCsv
  CurrentTypedArchiveLoadSummaryNotes   = $CurrentTypedArchiveLoadSummaryNotes
  CurrentMaterializationComparisonCsv   = $CurrentMaterializationComparisonCsv
  CurrentMaterializationComparisonNotes = $CurrentMaterializationComparisonNotes
  PreviousTargetSummaryCsv              = $PreviousTargetSummaryCsv
  CurrentTargetSummaryCsv               = $CurrentTargetSummaryCsv
}

Write-BenchmarkReviewRunManifest -Context $ReviewManifestContext

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
  Invoke-BenchmarkReviewValidationStep `
    -ReviewNotesPath $ReviewNotesPath `
    -Label $Label `
    -OutputDir $OutputDir `
    -ExpectedTrials $Trials `
    -RequireValidatedComparisons ([bool]$RequireValidatedComparisons) `
    -AllowSkippedTargetWorkloadReview ([bool]$SkipTargetWorkloadReview) `
    -ArtifactValidatorScript $ArtifactValidatorScript
}

if ($UpdateHistory) {
  Add-ReviewLine ""
  Add-ReviewLine "Benchmark history update"
  Add-ReviewLine "------------------------"
  Add-ReviewLine "status: running"
  Add-ReviewLine "history directory: $HistoryDir"

  Update-BenchmarkReviewHistory -Context $ReviewManifestContext

  Add-ReviewLine "status: completed"
  Add-ReviewLine "run history: $(Join-Path $HistoryDir "benchmark-run-history.csv")"
  Add-ReviewLine "low-selectivity performance history: $(Join-Path $HistoryDir "low-selectivity-performance-history.csv")"
  Add-ReviewLine "weakest workload history: $(Join-Path $HistoryDir "weakest-workload-history.csv")"
  Add-ReviewLine "count-only workload history: $(Join-Path $HistoryDir "count-only-workload-history.csv")"
  Add-ReviewLine "materialization mode history: $(Join-Path $HistoryDir "materialization-mode-history.csv")"
  Add-ReviewLine "target workload history: $(Join-Path $HistoryDir "target-workload-history.csv")"
  Add-ReviewLine "index footprint history: $(Join-Path $HistoryDir "index-footprint-history.csv")"
  Add-ReviewLine "baseline footprint history: $(Join-Path $HistoryDir "baseline-footprint-history.csv")"
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
  -CleanupFlatArtifacts ([bool]$CleanupFlatArtifacts) `
  -UpdateHistory ([bool]$UpdateHistory) `
  -HistoryDir $HistoryDir
