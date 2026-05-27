param(
  [string]$Label = "local",
  [string]$PreviousTrialSummaryCsv,
  [string]$CurrentTrialSummaryCsv,
  [string]$OutputDir = "benchmark_artifacts",
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"
$BenchmarkComparisonLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-comparison.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkComparisonLibrary)) {
  throw "benchmark comparison helper library was not found: $BenchmarkComparisonLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkComparisonLibrary

if ([string]::IsNullOrWhiteSpace($PreviousTrialSummaryCsv)) {
  throw "`-PreviousTrialSummaryCsv` is required"
}

if ([string]::IsNullOrWhiteSpace($CurrentTrialSummaryCsv)) {
  throw "`-CurrentTrialSummaryCsv` is required"
}

if (!(Test-Path -LiteralPath $PreviousTrialSummaryCsv)) {
  throw "previous low-gap trial summary CSV was not found: $PreviousTrialSummaryCsv"
}

if (!(Test-Path -LiteralPath $CurrentTrialSummaryCsv)) {
  throw "current low-gap trial summary CSV was not found: $CurrentTrialSummaryCsv"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$ComparisonCsvPath = Join-Path $OutputDir "low-gap-trial-comparison-$Label.csv"
$ComparisonNotesPath = Join-Path $OutputDir "low-gap-trial-comparison-$Label.txt"

$TreeBaselines = Get-BenchmarkTreeBaselineNames


function New-ComparisonRow {
  param(
    [string]$BaselineName,
    [object]$PreviousRow,
    [object]$CurrentRow
  )

  $PreviousTrialCount = Get-Metric -Row $PreviousRow -FieldName "trial_count"
  $CurrentTrialCount = Get-Metric -Row $CurrentRow -FieldName "trial_count"
  $TrialCountDelta = Get-Delta -CurrentValue $CurrentTrialCount -PreviousValue $PreviousTrialCount

  $PreviousMeanTiming = Get-Metric -Row $PreviousRow -FieldName "mean_low_mean_timing_ratio"
  $CurrentMeanTiming = Get-Metric -Row $CurrentRow -FieldName "mean_low_mean_timing_ratio"
  $MeanTimingDelta = Get-Delta -CurrentValue $CurrentMeanTiming -PreviousValue $PreviousMeanTiming
  $TimingClassification = Get-Classification -Delta $MeanTimingDelta -Threshold $NoiseThreshold

  $PreviousMinTiming = Get-Metric -Row $PreviousRow -FieldName "min_low_mean_timing_ratio"
  $CurrentMinTiming = Get-Metric -Row $CurrentRow -FieldName "min_low_mean_timing_ratio"
  $MinTimingDelta = Get-Delta -CurrentValue $CurrentMinTiming -PreviousValue $PreviousMinTiming

  $PreviousMaxTiming = Get-Metric -Row $PreviousRow -FieldName "max_low_mean_timing_ratio"
  $CurrentMaxTiming = Get-Metric -Row $CurrentRow -FieldName "max_low_mean_timing_ratio"
  $MaxTimingDelta = Get-Delta -CurrentValue $CurrentMaxTiming -PreviousValue $PreviousMaxTiming

  $PreviousRange = Get-Metric -Row $PreviousRow -FieldName "range_low_mean_timing_ratio"
  $CurrentRange = Get-Metric -Row $CurrentRow -FieldName "range_low_mean_timing_ratio"
  $RangeDelta = Get-Delta -CurrentValue $CurrentRange -PreviousValue $PreviousRange

  $PreviousStdDev = Get-Metric -Row $PreviousRow -FieldName "sample_stddev_low_mean_timing_ratio"
  $CurrentStdDev = Get-Metric -Row $CurrentRow -FieldName "sample_stddev_low_mean_timing_ratio"
  $StdDevDelta = Get-Delta -CurrentValue $CurrentStdDev -PreviousValue $PreviousStdDev

  $PreviousCandidate = Get-Metric -Row $PreviousRow -FieldName "mean_low_weighted_candidate_ratio"
  $CurrentCandidate = Get-Metric -Row $CurrentRow -FieldName "mean_low_weighted_candidate_ratio"
  $CandidateDelta = Get-Delta -CurrentValue $CurrentCandidate -PreviousValue $PreviousCandidate

  $PreviousAvoidance = Get-Metric -Row $PreviousRow -FieldName "mean_low_weighted_avoidance_ratio"
  $CurrentAvoidance = Get-Metric -Row $CurrentRow -FieldName "mean_low_weighted_avoidance_ratio"
  $AvoidanceDelta = Get-Delta -CurrentValue $CurrentAvoidance -PreviousValue $PreviousAvoidance

  return [PSCustomObject]@{
    BaselineName           = $BaselineName
    PreviousTrialCount     = $PreviousTrialCount
    CurrentTrialCount      = $CurrentTrialCount
    TrialCountDelta        = $TrialCountDelta
    PreviousMeanTiming     = $PreviousMeanTiming
    CurrentMeanTiming      = $CurrentMeanTiming
    MeanTimingDelta        = $MeanTimingDelta
    TimingClassification   = $TimingClassification
    PreviousMinTiming      = $PreviousMinTiming
    CurrentMinTiming       = $CurrentMinTiming
    MinTimingDelta         = $MinTimingDelta
    PreviousMaxTiming      = $PreviousMaxTiming
    CurrentMaxTiming       = $CurrentMaxTiming
    MaxTimingDelta         = $MaxTimingDelta
    PreviousRange          = $PreviousRange
    CurrentRange           = $CurrentRange
    RangeDelta             = $RangeDelta
    PreviousStdDev         = $PreviousStdDev
    CurrentStdDev          = $CurrentStdDev
    StdDevDelta            = $StdDevDelta
    PreviousCandidate      = $PreviousCandidate
    CurrentCandidate       = $CurrentCandidate
    CandidateDelta         = $CandidateDelta
    PreviousAvoidance      = $PreviousAvoidance
    CurrentAvoidance       = $CurrentAvoidance
    AvoidanceDelta         = $AvoidanceDelta
    PreviousClassification = Get-TextField -Row $PreviousRow -FieldName "classification"
    CurrentClassification  = Get-TextField -Row $CurrentRow -FieldName "classification"
  }
}

$PreviousResolvedPath = (Resolve-Path -LiteralPath $PreviousTrialSummaryCsv).Path
$CurrentResolvedPath = (Resolve-Path -LiteralPath $CurrentTrialSummaryCsv).Path

$PreviousRows = @(Import-Csv -LiteralPath $PreviousResolvedPath)
$CurrentRows = @(Import-Csv -LiteralPath $CurrentResolvedPath)

$ComparisonObjects = @(New-BenchmarkBaselineComparisonObjects `
    -PreviousRows $PreviousRows `
    -CurrentRows $CurrentRows `
    -BaselineNames $TreeBaselines `
    -RowFactory {
    param(
      [string]$BaselineName,
      [object]$PreviousRow,
      [object]$CurrentRow
    )

    New-ComparisonRow `
      -BaselineName $BaselineName `
      -PreviousRow $PreviousRow `
      -CurrentRow $CurrentRow
  })

$ComparisonRows = @()

foreach ($Comparison in $ComparisonObjects) {
  $ComparisonRows += , @(
    $Comparison.BaselineName,
    $(Format-InvariantDouble -Value $Comparison.PreviousTrialCount),
    $(Format-InvariantDouble -Value $Comparison.CurrentTrialCount),
    $(Format-InvariantDouble -Value $Comparison.TrialCountDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousMeanTiming),
    $(Format-InvariantDouble -Value $Comparison.CurrentMeanTiming),
    $(Format-InvariantDouble -Value $Comparison.MeanTimingDelta),
    $Comparison.TimingClassification,
    $Comparison.PreviousClassification,
    $Comparison.CurrentClassification,
    $(Format-InvariantDouble -Value $Comparison.PreviousMinTiming),
    $(Format-InvariantDouble -Value $Comparison.CurrentMinTiming),
    $(Format-InvariantDouble -Value $Comparison.MinTimingDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousMaxTiming),
    $(Format-InvariantDouble -Value $Comparison.CurrentMaxTiming),
    $(Format-InvariantDouble -Value $Comparison.MaxTimingDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousRange),
    $(Format-InvariantDouble -Value $Comparison.CurrentRange),
    $(Format-InvariantDouble -Value $Comparison.RangeDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousStdDev),
    $(Format-InvariantDouble -Value $Comparison.CurrentStdDev),
    $(Format-InvariantDouble -Value $Comparison.StdDevDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousCandidate),
    $(Format-InvariantDouble -Value $Comparison.CurrentCandidate),
    $(Format-InvariantDouble -Value $Comparison.CandidateDelta),
    $(Format-InvariantDouble -Value $Comparison.PreviousAvoidance),
    $(Format-InvariantDouble -Value $Comparison.CurrentAvoidance),
    $(Format-InvariantDouble -Value $Comparison.AvoidanceDelta)
  )
}

Write-CsvDocument `
  -Path $ComparisonCsvPath `
  -Header @(
  "baseline_name",
  "previous_trial_count",
  "current_trial_count",
  "trial_count_delta",
  "previous_mean_low_mean_timing_ratio",
  "current_mean_low_mean_timing_ratio",
  "mean_low_mean_timing_delta",
  "timing_classification",
  "previous_classification",
  "current_classification",
  "previous_min_low_mean_timing_ratio",
  "current_min_low_mean_timing_ratio",
  "min_low_mean_timing_delta",
  "previous_max_low_mean_timing_ratio",
  "current_max_low_mean_timing_ratio",
  "max_low_mean_timing_delta",
  "previous_range_low_mean_timing_ratio",
  "current_range_low_mean_timing_ratio",
  "range_low_mean_timing_delta",
  "previous_sample_stddev_low_mean_timing_ratio",
  "current_sample_stddev_low_mean_timing_ratio",
  "sample_stddev_low_mean_timing_delta",
  "previous_mean_low_weighted_candidate_ratio",
  "current_mean_low_weighted_candidate_ratio",
  "mean_low_weighted_candidate_delta",
  "previous_mean_low_weighted_avoidance_ratio",
  "current_mean_low_weighted_avoidance_ratio",
  "mean_low_weighted_avoidance_delta"
) `
  -Rows $ComparisonRows

Set-Utf8Text -Path $ComparisonNotesPath -Text "Low-gap trial summary comparison`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "================================`r`n`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous trial summary: $PreviousTrialSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous resolved path: $PreviousResolvedPath`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current trial summary: $CurrentTrialSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current resolved path: $CurrentResolvedPath`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Timing ratio meaning: baseline elapsed / FSE elapsed; higher is better for FSE.`r`n"

foreach ($Comparison in $ComparisonObjects) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n$($Comparison.BaselineName)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text ("-" * $Comparison.BaselineName.Length)
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous trial count: $(Format-InvariantDouble -Value $Comparison.PreviousTrialCount)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current trial count: $(Format-InvariantDouble -Value $Comparison.CurrentTrialCount)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous mean low timing: $(Format-InvariantDouble -Value $Comparison.PreviousMeanTiming)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current mean low timing: $(Format-InvariantDouble -Value $Comparison.CurrentMeanTiming)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean low timing delta: $(Format-InvariantDouble -Value $Comparison.MeanTimingDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "classification: $($Comparison.TimingClassification)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous range: $(Format-InvariantDouble -Value $Comparison.PreviousRange)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current range: $(Format-InvariantDouble -Value $Comparison.CurrentRange)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "range delta: $(Format-InvariantDouble -Value $Comparison.RangeDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "previous sample stddev: $(Format-InvariantDouble -Value $Comparison.PreviousStdDev)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "current sample stddev: $(Format-InvariantDouble -Value $Comparison.CurrentStdDev)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "sample stddev delta: $(Format-InvariantDouble -Value $Comparison.StdDevDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean weighted candidate delta: $(Format-InvariantDouble -Value $Comparison.CandidateDelta)`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "mean weighted avoidance delta: $(Format-InvariantDouble -Value $Comparison.AvoidanceDelta)`r`n"
}

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Use this comparison for aggregate low-selectivity movement across repeated trials.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Prefer improvements beyond the noise threshold before accepting query performance changes.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "A timing improvement is stronger when candidate and avoidance ratios do not regress unexpectedly.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "If range or sample stddev grows sharply, rerun before making a performance decision.`r`n"

Write-Host ""
Write-Host "Low-gap trial comparison artifacts written:"
Write-Host "  $ComparisonCsvPath"
Write-Host "  $ComparisonNotesPath"