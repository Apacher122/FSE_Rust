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
  [string]$TargetWorkloadName = "",
  [string]$PreviousTargetSummaryCsv = "",
  [switch]$SkipTargetWorkloadReview,
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

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

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$ScriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path

$SmallBenchmarkScript = Join-Path $ScriptDirectory "run-small-benchmark.ps1"
$LowGapTrialsScript = Join-Path $ScriptDirectory "run-low-gap-trials.ps1"
$AggregateComparatorScript = Join-Path $ScriptDirectory "compare-low-gap-trial-summary.ps1"
$WorkloadComparatorScript = Join-Path $ScriptDirectory "compare-low-gap-workload-summary.ps1"
$TargetSummaryScript = Join-Path $ScriptDirectory "summarize-target-workload-trials.ps1"
$TargetComparatorScript = Join-Path $ScriptDirectory "compare-target-workload-summary.ps1"

$ReviewNotesPath = Join-Path $OutputDir "low-gap-review-notes-$Label.txt"
$ReviewManifestPath = Join-Path $OutputDir "low-gap-review-manifest-$Label.txt"
$CurrentTrialSummaryCsv = Join-Path $OutputDir "low-gap-trial-summary-$Label.csv"
$CurrentWorkloadSummaryCsv = Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv"
$CurrentTargetDetailCsv = Join-Path $OutputDir "target-workload-trial-details-$Label.csv"
$CurrentTargetSummaryCsv = Join-Path $OutputDir "target-workload-trial-summary-$Label.csv"
$CurrentTargetNotesPath = Join-Path $OutputDir "target-workload-trial-notes-$Label.txt"

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$InvariantCulture = [System.Globalization.CultureInfo]::InvariantCulture

function Set-Utf8Text {
  param(
    [string]$Path,
    [string]$Text
  )

  [System.IO.File]::WriteAllText($Path, $Text, $Utf8NoBom)
}

function Add-Utf8Text {
  param(
    [string]$Path,
    [string]$Text
  )

  [System.IO.File]::AppendAllText($Path, $Text, $Utf8NoBom)
}

function Format-InvariantDouble {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return "unavailable"
  }

  try {
    $DoubleValue = [System.Convert]::ToDouble($Value, $InvariantCulture)
    return $DoubleValue.ToString("0.000000", $InvariantCulture)
  }
  catch {
    return "unavailable"
  }
}

function Require-Script {
  param(
    [string]$Path
  )

  if (!(Test-Path -LiteralPath $Path)) {
    throw "required script was not found: $Path"
  }
}

function Resolve-OptionalPreviousPath {
  param(
    [string]$ExplicitPath,
    [string]$DefaultPath
  )

  if (![string]::IsNullOrWhiteSpace($ExplicitPath)) {
    return $ExplicitPath
  }

  if (![string]::IsNullOrWhiteSpace($PreviousLabel)) {
    return $DefaultPath
  }

  return ""
}

function Get-InputStatus {
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

function Add-ReviewLine {
  param(
    [string]$Text
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "$Text`r`n"
}

function Get-ArtifactState {
  param(
    [string]$Path,
    [bool]$Skipped
  )

  if ($Skipped) {
    return "skipped"
  }

  if (Test-Path -LiteralPath $Path) {
    return "found"
  }

  return "missing"
}

function Add-ManifestArtifactLine {
  param(
    [string]$Name,
    [string]$Path,
    [bool]$Skipped,
    [string]$Note = ""
  )

  $State = Get-ArtifactState -Path $Path -Skipped $Skipped

  if ([string]::IsNullOrWhiteSpace($Note)) {
    Add-Utf8Text -Path $ReviewManifestPath -Text "$State | $Name | $Path`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewManifestPath -Text "$State | $Name | $Path | $Note`r`n"
  }
}

function Add-PreviousInputStatusLines {
  param(
    [object]$SingleRunInput,
    [object]$AggregateInput,
    [object]$WorkloadInput,
    [object]$TargetInput
  )

  Add-ReviewLine "Previous input status"
  Add-ReviewLine "---------------------"
  Add-ReviewLine "single-run low-selectivity CSV status: $($SingleRunInput.Status)"
  Add-ReviewLine "aggregate trial summary status: $($AggregateInput.Status)"
  Add-ReviewLine "workload trial summary status: $($WorkloadInput.Status)"
  Add-ReviewLine "target workload trial summary status: $($TargetInput.Status)"

  if ($SingleRunInput.Exists) {
    Add-ReviewLine "single-run low-selectivity CSV resolved path: $($SingleRunInput.ResolvedPath)"
  }

  if ($AggregateInput.Exists) {
    Add-ReviewLine "aggregate trial summary resolved path: $($AggregateInput.ResolvedPath)"
  }

  if ($WorkloadInput.Exists) {
    Add-ReviewLine "workload trial summary resolved path: $($WorkloadInput.ResolvedPath)"
  }

  if ($TargetInput.Exists) {
    Add-ReviewLine "target workload trial summary resolved path: $($TargetInput.ResolvedPath)"
  }

  Add-ReviewLine ""
}

function Get-AggregateComparisonSkipReason {
  if ([string]::IsNullOrWhiteSpace($PreviousTrialSummaryCsv)) {
    return "previous aggregate trial summary was not provided"
  }

  if (!(Test-Path -LiteralPath $PreviousTrialSummaryCsv)) {
    return "previous aggregate trial summary was not found"
  }

  if (!(Test-Path -LiteralPath $CurrentTrialSummaryCsv)) {
    return "current aggregate trial summary was not found"
  }

  return ""
}

function Get-WorkloadComparisonSkipReason {
  if ([string]::IsNullOrWhiteSpace($PreviousWorkloadSummaryCsv)) {
    return "previous workload trial summary was not provided"
  }

  if (!(Test-Path -LiteralPath $PreviousWorkloadSummaryCsv)) {
    return "previous workload trial summary was not found"
  }

  if (!(Test-Path -LiteralPath $CurrentWorkloadSummaryCsv)) {
    return "current workload trial summary was not found"
  }

  return ""
}

function Get-TargetComparisonSkipReason {
  if ($SkipTargetWorkloadReview) {
    return "target workload review was disabled"
  }

  if ([string]::IsNullOrWhiteSpace($PreviousTargetSummaryCsv)) {
    return "previous target workload summary was not provided"
  }

  if (!(Test-Path -LiteralPath $PreviousTargetSummaryCsv)) {
    return "previous target workload summary was not found"
  }

  if (!(Test-Path -LiteralPath $CurrentTargetSummaryCsv)) {
    return "current target workload summary was not found"
  }

  return ""
}

function Write-ReviewManifest {
  $AggregateComparisonSkipReason = Get-AggregateComparisonSkipReason
  $WorkloadComparisonSkipReason = Get-WorkloadComparisonSkipReason
  $TargetComparisonSkipReason = Get-TargetComparisonSkipReason

  $AggregateComparisonSkipped = ![string]::IsNullOrWhiteSpace($AggregateComparisonSkipReason)
  $WorkloadComparisonSkipped = ![string]::IsNullOrWhiteSpace($WorkloadComparisonSkipReason)
  $TargetSummarySkipped = [bool]$SkipTargetWorkloadReview
  $TargetComparisonSkipped = ![string]::IsNullOrWhiteSpace($TargetComparisonSkipReason)

  Set-Utf8Text -Path $ReviewManifestPath -Text "Low-gap review artifact manifest`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "================================`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Label: $Label`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Max depth: $EffectiveMaxDepth`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Trials: $Trials`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Iterations: $Iterations`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Target workload: $TargetWorkloadName`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "state | artifact | path | note`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "------|----------|------|-----`r`n"

  Add-ManifestArtifactLine `
    -Name "benchmark output" `
    -Path (Join-Path $OutputDir "benchmark-output-$Label.txt") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "debug output" `
    -Path (Join-Path $OutputDir "debug-output-$Label.txt") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "summary CSV" `
    -Path (Join-Path $OutputDir "summary-$Label.csv") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "workloads CSV" `
    -Path (Join-Path $OutputDir "workloads-$Label.csv") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-selectivity gap CSV" `
    -Path (Join-Path $OutputDir "low-selectivity-gap-$Label.csv") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-gap regression notes" `
    -Path (Join-Path $OutputDir "low-gap-regression-notes-$Label.txt") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-gap trial details" `
    -Path (Join-Path $OutputDir "low-gap-trial-details-$Label.csv") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-gap trial summary" `
    -Path (Join-Path $OutputDir "low-gap-trial-summary-$Label.csv") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-gap trial notes" `
    -Path (Join-Path $OutputDir "low-gap-trial-notes-$Label.txt") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-gap workload trial details" `
    -Path (Join-Path $OutputDir "low-gap-workload-trial-details-$Label.csv") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-gap workload trial summary" `
    -Path (Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv") `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "aggregate trial comparison CSV" `
    -Path (Join-Path $OutputDir "low-gap-trial-comparison-$Label.csv") `
    -Skipped $AggregateComparisonSkipped `
    -Note $AggregateComparisonSkipReason

  Add-ManifestArtifactLine `
    -Name "aggregate trial comparison notes" `
    -Path (Join-Path $OutputDir "low-gap-trial-comparison-$Label.txt") `
    -Skipped $AggregateComparisonSkipped `
    -Note $AggregateComparisonSkipReason

  Add-ManifestArtifactLine `
    -Name "workload trial comparison CSV" `
    -Path (Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.csv") `
    -Skipped $WorkloadComparisonSkipped `
    -Note $WorkloadComparisonSkipReason

  Add-ManifestArtifactLine `
    -Name "workload trial comparison notes" `
    -Path (Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.txt") `
    -Skipped $WorkloadComparisonSkipped `
    -Note $WorkloadComparisonSkipReason

  Add-ManifestArtifactLine `
    -Name "target workload trial details" `
    -Path (Join-Path $OutputDir "target-workload-trial-details-$Label.csv") `
    -Skipped $TargetSummarySkipped `
    -Note $(if ($TargetSummarySkipped) { "target workload review was disabled" } else { "" })

  Add-ManifestArtifactLine `
    -Name "target workload trial summary" `
    -Path (Join-Path $OutputDir "target-workload-trial-summary-$Label.csv") `
    -Skipped $TargetSummarySkipped `
    -Note $(if ($TargetSummarySkipped) { "target workload review was disabled" } else { "" })

  Add-ManifestArtifactLine `
    -Name "target workload trial notes" `
    -Path (Join-Path $OutputDir "target-workload-trial-notes-$Label.txt") `
    -Skipped $TargetSummarySkipped `
    -Note $(if ($TargetSummarySkipped) { "target workload review was disabled" } else { "" })

  Add-ManifestArtifactLine `
    -Name "target workload trial comparison CSV" `
    -Path (Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv") `
    -Skipped $TargetComparisonSkipped `
    -Note $TargetComparisonSkipReason

  Add-ManifestArtifactLine `
    -Name "target workload trial comparison notes" `
    -Path (Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt") `
    -Skipped $TargetComparisonSkipped `
    -Note $TargetComparisonSkipReason

  Add-ManifestArtifactLine `
    -Name "low-gap review notes" `
    -Path $ReviewNotesPath `
    -Skipped $false

  Add-ManifestArtifactLine `
    -Name "low-gap review manifest" `
    -Path $ReviewManifestPath `
    -Skipped $false

  Add-Utf8Text -Path $ReviewManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "Manifest interpretation`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "-----------------------`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "found: the artifact exists after the review run`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "missing: the artifact was expected but was not found`r`n"
  Add-Utf8Text -Path $ReviewManifestPath -Text "skipped: the artifact was intentionally not generated because an input or flag made that step unavailable`r`n"
}

Require-Script -Path $SmallBenchmarkScript
Require-Script -Path $LowGapTrialsScript
Require-Script -Path $AggregateComparatorScript
Require-Script -Path $WorkloadComparatorScript

if (!$SkipTargetWorkloadReview) {
  Require-Script -Path $TargetSummaryScript
  Require-Script -Path $TargetComparatorScript
}

$PreviousLowSelectivityGapCsv = Resolve-OptionalPreviousPath `
  -ExplicitPath $PreviousLowSelectivityGapCsv `
  -DefaultPath (Join-Path $OutputDir "low-selectivity-gap-$PreviousLabel.csv")

$PreviousTrialSummaryCsv = Resolve-OptionalPreviousPath `
  -ExplicitPath $PreviousTrialSummaryCsv `
  -DefaultPath (Join-Path $OutputDir "low-gap-trial-summary-$PreviousLabel.csv")

$PreviousWorkloadSummaryCsv = Resolve-OptionalPreviousPath `
  -ExplicitPath $PreviousWorkloadSummaryCsv `
  -DefaultPath (Join-Path $OutputDir "low-gap-workload-trial-summary-$PreviousLabel.csv")

$PreviousTargetSummaryCsv = Resolve-OptionalPreviousPath `
  -ExplicitPath $PreviousTargetSummaryCsv `
  -DefaultPath (Join-Path $OutputDir "target-workload-trial-summary-$PreviousLabel.csv")

$PreviousLowSelectivityGapInput = Get-InputStatus -Path $PreviousLowSelectivityGapCsv
$PreviousTrialSummaryInput = Get-InputStatus -Path $PreviousTrialSummaryCsv
$PreviousWorkloadSummaryInput = Get-InputStatus -Path $PreviousWorkloadSummaryCsv
$PreviousTargetSummaryInput = Get-InputStatus -Path $PreviousTargetSummaryCsv

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
Add-ReviewLine "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)"
Add-ReviewLine "Target workload review skipped: $SkipTargetWorkloadReview"
Add-ReviewLine "Target workload name: $TargetWorkloadName"
Add-ReviewLine "Previous low-selectivity gap CSV: $PreviousLowSelectivityGapCsv"
Add-ReviewLine "Previous aggregate trial summary CSV: $PreviousTrialSummaryCsv"
Add-ReviewLine "Previous workload trial summary CSV: $PreviousWorkloadSummaryCsv"
Add-ReviewLine "Previous target workload summary CSV: $PreviousTargetSummaryCsv"
Add-ReviewLine ""

Add-PreviousInputStatusLines `
  -SingleRunInput $PreviousLowSelectivityGapInput `
  -AggregateInput $PreviousTrialSummaryInput `
  -WorkloadInput $PreviousWorkloadSummaryInput `
  -TargetInput $PreviousTargetSummaryInput

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

Write-Host ""
Write-Host "Running target workload trial comparison"

Add-ReviewLine "Target workload trial comparison"
Add-ReviewLine "--------------------------------"

if ($SkipTargetWorkloadReview) {
  Write-Host "  skipped: target workload review was disabled"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: -SkipTargetWorkloadReview was passed"
}
elseif ([string]::IsNullOrWhiteSpace($PreviousTargetSummaryCsv)) {
  Write-Host "  skipped: previous target workload summary was not provided"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: previous target workload summary was not provided"
}
elseif (!(Test-Path -LiteralPath $PreviousTargetSummaryCsv)) {
  Write-Host "  skipped: previous target workload summary was not found"
  Write-Host "  $PreviousTargetSummaryCsv"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: previous target workload summary was not found"
}
elseif (!(Test-Path -LiteralPath $CurrentTargetSummaryCsv)) {
  Write-Host "  skipped: current target workload summary was not found"
  Write-Host "  $CurrentTargetSummaryCsv"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: current target workload summary was not found"
}
else {
  & $TargetComparatorScript `
    -Label $Label `
    -PreviousTargetSummaryCsv $PreviousTargetSummaryCsv `
    -CurrentTargetSummaryCsv $CurrentTargetSummaryCsv `
    -OutputDir $OutputDir `
    -NoiseThreshold $NoiseThreshold

  if ($LASTEXITCODE -ne 0) {
    throw "target workload trial comparison failed with exit code $LASTEXITCODE"
  }

  Add-ReviewLine "status: completed"
  Add-ReviewLine "comparison CSV: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv")"
  Add-ReviewLine "comparison notes: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt")"
}

Add-ReviewLine ""

Write-Host ""
Write-Host "Running aggregate trial comparison"

Add-ReviewLine "Aggregate trial comparison"
Add-ReviewLine "--------------------------"

if ([string]::IsNullOrWhiteSpace($PreviousTrialSummaryCsv)) {
  Write-Host "  skipped: previous aggregate trial summary was not provided"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: previous aggregate trial summary was not provided"
}
elseif (!(Test-Path -LiteralPath $PreviousTrialSummaryCsv)) {
  Write-Host "  skipped: previous aggregate trial summary was not found"
  Write-Host "  $PreviousTrialSummaryCsv"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: previous aggregate trial summary was not found"
}
elseif (!(Test-Path -LiteralPath $CurrentTrialSummaryCsv)) {
  Write-Host "  skipped: current aggregate trial summary was not found"
  Write-Host "  $CurrentTrialSummaryCsv"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: current aggregate trial summary was not found"
}
else {
  & $AggregateComparatorScript `
    -Label $Label `
    -PreviousTrialSummaryCsv $PreviousTrialSummaryCsv `
    -CurrentTrialSummaryCsv $CurrentTrialSummaryCsv `
    -OutputDir $OutputDir `
    -NoiseThreshold $NoiseThreshold

  if ($LASTEXITCODE -ne 0) {
    throw "aggregate trial comparison failed with exit code $LASTEXITCODE"
  }

  Add-ReviewLine "status: completed"
  Add-ReviewLine "comparison CSV: $(Join-Path $OutputDir "low-gap-trial-comparison-$Label.csv")"
  Add-ReviewLine "comparison notes: $(Join-Path $OutputDir "low-gap-trial-comparison-$Label.txt")"
}

Add-ReviewLine ""

Write-Host ""
Write-Host "Running workload trial comparison"

Add-ReviewLine "Workload trial comparison"
Add-ReviewLine "-------------------------"

if ([string]::IsNullOrWhiteSpace($PreviousWorkloadSummaryCsv)) {
  Write-Host "  skipped: previous workload trial summary was not provided"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: previous workload trial summary was not provided"
}
elseif (!(Test-Path -LiteralPath $PreviousWorkloadSummaryCsv)) {
  Write-Host "  skipped: previous workload trial summary was not found"
  Write-Host "  $PreviousWorkloadSummaryCsv"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: previous workload trial summary was not found"
}
elseif (!(Test-Path -LiteralPath $CurrentWorkloadSummaryCsv)) {
  Write-Host "  skipped: current workload trial summary was not found"
  Write-Host "  $CurrentWorkloadSummaryCsv"
  Add-ReviewLine "status: skipped"
  Add-ReviewLine "reason: current workload trial summary was not found"
}
else {
  & $WorkloadComparatorScript `
    -Label $Label `
    -PreviousWorkloadSummaryCsv $PreviousWorkloadSummaryCsv `
    -CurrentWorkloadSummaryCsv $CurrentWorkloadSummaryCsv `
    -OutputDir $OutputDir `
    -NoiseThreshold $NoiseThreshold

  if ($LASTEXITCODE -ne 0) {
    throw "workload trial comparison failed with exit code $LASTEXITCODE"
  }

  Add-ReviewLine "status: completed"
  Add-ReviewLine "comparison CSV: $(Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.csv")"
  Add-ReviewLine "comparison notes: $(Join-Path $OutputDir "low-gap-workload-trial-comparison-$Label.txt")"
}

Add-ReviewLine ""
Add-ReviewLine "Comparison availability summary"
Add-ReviewLine "-------------------------------"

if ($PreviousLowSelectivityGapInput.Exists) {
  Add-ReviewLine "single-run low-gap comparison: completed"
}
else {
  Add-ReviewLine "single-run low-gap comparison: unavailable"
  Add-ReviewLine "single-run low-gap comparison reason: previous low-selectivity gap CSV was $($PreviousLowSelectivityGapInput.Status)"
}

if ($PreviousTrialSummaryInput.Exists -and (Test-Path -LiteralPath $CurrentTrialSummaryCsv)) {
  Add-ReviewLine "aggregate repeated-trial comparison: completed"
}
elseif (!$PreviousTrialSummaryInput.Exists) {
  Add-ReviewLine "aggregate repeated-trial comparison: skipped"
  Add-ReviewLine "aggregate repeated-trial comparison reason: previous aggregate trial summary was $($PreviousTrialSummaryInput.Status)"
}
else {
  Add-ReviewLine "aggregate repeated-trial comparison: skipped"
  Add-ReviewLine "aggregate repeated-trial comparison reason: current aggregate trial summary was missing"
}

if ($PreviousWorkloadSummaryInput.Exists -and (Test-Path -LiteralPath $CurrentWorkloadSummaryCsv)) {
  Add-ReviewLine "workload repeated-trial comparison: completed"
}
elseif (!$PreviousWorkloadSummaryInput.Exists) {
  Add-ReviewLine "workload repeated-trial comparison: skipped"
  Add-ReviewLine "workload repeated-trial comparison reason: previous workload trial summary was $($PreviousWorkloadSummaryInput.Status)"
}
else {
  Add-ReviewLine "workload repeated-trial comparison: skipped"
  Add-ReviewLine "workload repeated-trial comparison reason: current workload trial summary was missing"
}

if ($SkipTargetWorkloadReview) {
  Add-ReviewLine "target workload summary: skipped"
  Add-ReviewLine "target workload summary reason: target workload review was disabled"
  Add-ReviewLine "target workload comparison: skipped"
  Add-ReviewLine "target workload comparison reason: target workload review was disabled"
}
else {
  if (Test-Path -LiteralPath $CurrentTargetSummaryCsv) {
    Add-ReviewLine "target workload summary: completed"
  }
  else {
    Add-ReviewLine "target workload summary: unavailable"
    Add-ReviewLine "target workload summary reason: current target workload summary was missing"
  }

  if ($PreviousTargetSummaryInput.Exists -and (Test-Path -LiteralPath $CurrentTargetSummaryCsv)) {
    Add-ReviewLine "target workload comparison: completed"
  }
  elseif (!$PreviousTargetSummaryInput.Exists) {
    Add-ReviewLine "target workload comparison: skipped"
    Add-ReviewLine "target workload comparison reason: previous target workload summary was $($PreviousTargetSummaryInput.Status)"
  }
  else {
    Add-ReviewLine "target workload comparison: skipped"
    Add-ReviewLine "target workload comparison reason: current target workload summary was missing"
  }
}

Add-ReviewLine ""
Add-ReviewLine "Review artifact summary"
Add-ReviewLine "-----------------------"
Add-ReviewLine "benchmark output: $(Join-Path $OutputDir "benchmark-output-$Label.txt")"
Add-ReviewLine "debug output: $(Join-Path $OutputDir "debug-output-$Label.txt")"
Add-ReviewLine "summary CSV: $(Join-Path $OutputDir "summary-$Label.csv")"
Add-ReviewLine "workloads CSV: $(Join-Path $OutputDir "workloads-$Label.csv")"
Add-ReviewLine "low-selectivity gap CSV: $(Join-Path $OutputDir "low-selectivity-gap-$Label.csv")"
Add-ReviewLine "low-gap regression notes: $(Join-Path $OutputDir "low-gap-regression-notes-$Label.txt")"
Add-ReviewLine "low-gap trial details: $(Join-Path $OutputDir "low-gap-trial-details-$Label.csv")"
Add-ReviewLine "low-gap trial summary: $(Join-Path $OutputDir "low-gap-trial-summary-$Label.csv")"
Add-ReviewLine "low-gap trial notes: $(Join-Path $OutputDir "low-gap-trial-notes-$Label.txt")"
Add-ReviewLine "low-gap workload trial details: $(Join-Path $OutputDir "low-gap-workload-trial-details-$Label.csv")"
Add-ReviewLine "low-gap workload trial summary: $(Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv")"
Add-ReviewLine "target workload trial details: $(Join-Path $OutputDir "target-workload-trial-details-$Label.csv")"
Add-ReviewLine "target workload trial summary: $(Join-Path $OutputDir "target-workload-trial-summary-$Label.csv")"
Add-ReviewLine "target workload trial notes: $(Join-Path $OutputDir "target-workload-trial-notes-$Label.txt")"
Add-ReviewLine "target workload trial comparison CSV: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv")"
Add-ReviewLine "target workload trial comparison notes: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt")"
Add-ReviewLine "low-gap review notes: $ReviewNotesPath"
Add-ReviewLine "low-gap review manifest: $ReviewManifestPath"
Add-ReviewLine ""
Add-ReviewLine "Decision guidance"
Add-ReviewLine "-----------------"
Add-ReviewLine "Use this review script for query-performance commits."
Add-ReviewLine "Use aggregate comparison artifacts for low-selectivity bucket movement."
Add-ReviewLine "Use workload comparison artifacts for weakest-workload movement."
Add-ReviewLine "Use target-workload comparison artifacts for $TargetWorkloadName movement."
Add-ReviewLine "Do not accept a performance commit based on one noisy single-run result."

Write-ReviewManifest

Write-Host ""
Write-Host "Low-gap review complete:"
Write-Host "  $ReviewNotesPath"
Write-Host "  $ReviewManifestPath"