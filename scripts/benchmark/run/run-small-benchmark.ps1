param(
    [string]$Label = "local",
    [ValidateSet("small", "large")]
    [string]$Dataset = "small",
    [ValidateRange(0, 2147483647)]
    [int]$MaxDepth = 0,
    [int]$Iterations = 10000,
    [string]$OutputDir = "benchmark_artifacts",
    [string]$PreviousLowSelectivityGapCsv = "",
    [double]$LowGapNoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$BenchmarkCsvLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-csv.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
    throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

. $BenchmarkCsvLibrary

$BenchmarkLowGapLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-low-gap.ps1"

if (!(Test-Path -LiteralPath $BenchmarkLowGapLibrary)) {
    throw "benchmark low-gap helper library was not found: $BenchmarkLowGapLibrary"
}

. $BenchmarkLowGapLibrary
if ($MaxDepth -gt 0) {
    $EffectiveMaxDepth = $MaxDepth
}
elseif ($Dataset -eq "large") {
    $EffectiveMaxDepth = 16
}
else {
    $EffectiveMaxDepth = 8
}

$EffectiveMaxDepthText = $EffectiveMaxDepth.ToString()
$BenchmarkScriptRoot = Split-Path -Parent $PSScriptRoot

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TextOutputPath = Join-Path $OutputDir "benchmark-output-$Label.txt"
$DebugOutputPath = Join-Path $OutputDir "debug-output-$Label.txt"
$SummaryCsvPath = Join-Path $OutputDir "summary-$Label.csv"
$WorkloadsCsvPath = Join-Path $OutputDir "workloads-$Label.csv"
$CountOnlyWorkloadSummaryCsvPath = Join-Path $OutputDir "count-only-workload-summary-$Label.csv"
$CountOnlyWorkloadSummaryNotesPath = Join-Path $OutputDir "count-only-workload-summary-$Label.txt"
$MaterializationModeSummaryCsvPath = Join-Path $OutputDir "materialization-mode-summary-$Label.csv"
$MaterializationModeSummaryNotesPath = Join-Path $OutputDir "materialization-mode-summary-$Label.txt"
$ExistenceTimingSummaryCsvPath = Join-Path $OutputDir "existence-timing-summary-$Label.csv"
$ExistenceTimingSummaryNotesPath = Join-Path $OutputDir "existence-timing-summary-$Label.txt"
$TypedIndexedComparisonSummaryCsvPath = Join-Path $OutputDir "typed-indexed-comparison-summary-$Label.csv"
$TypedIndexedComparisonSummaryNotesPath = Join-Path $OutputDir "typed-indexed-comparison-summary-$Label.txt"
$LowSelectivityGapCsvPath = Join-Path $OutputDir "low-selectivity-gap-$Label.csv"
$LowGapRegressionNotesPath = Join-Path $OutputDir "low-gap-regression-notes-$Label.txt"

$CountOnlyWorkloadSummaryScript = Join-Path (Join-Path $BenchmarkScriptRoot "summarize") "summarize-count-only-workload-summary.ps1"
$MaterializationModeSummaryScript = Join-Path (Join-Path $BenchmarkScriptRoot "summarize") "summarize-materialization-mode-summary.ps1"
$ExistenceTimingSummaryScript = Join-Path (Join-Path $BenchmarkScriptRoot "summarize") "summarize-existence-timing-summary.ps1"
$TypedIndexedComparisonSummaryScript = Join-Path (Join-Path $BenchmarkScriptRoot "summarize") "summarize-typed-indexed-comparison-summary.ps1"


function Write-BenchmarkSection {
    param(
        [string]$Path,
        [string]$Title,
        [switch]$First
    )

    if ($First) {
        Set-Utf8Text -Path $Path -Text "$Title`r`n"
        return
    }

    Add-Utf8Text -Path $Path -Text "`r`n---`r`n`r`n$Title`r`n"
}

function Invoke-BenchmarkCommand {
    param(
        [string]$OutputPath,
        [string]$Title,
        [string[]]$CargoArguments,
        [switch]$First
    )

    Write-BenchmarkSection -Path $OutputPath -Title $Title -First:$First

    # capture native process output as strings, mirror it to the terminal,
    # then append it as explicit utf8 so uploads stay readable
    $CommandOutput = & cargo @CargoArguments 2>&1
    $ExitCode = $LASTEXITCODE

    foreach ($Line in $CommandOutput) {
        $LineText = $Line.ToString()
        Write-Host $LineText
        Add-Utf8Text -Path $OutputPath -Text "$LineText`r`n"
    }

    if ($ExitCode -ne 0) {
        throw "benchmark command failed for section '$Title' with exit code $ExitCode"
    }
}

function Get-PreviousCsvLoadStatus {
    param(
        [string]$PreviousPath
    )

    if ([string]::IsNullOrWhiteSpace($PreviousPath)) {
        return [PSCustomObject]@{
            Status       = "not provided"
            Exists       = $false
            Loaded       = $false
            ResolvedPath = ""
            Rows         = @()
        }
    }

    if (!(Test-Path -LiteralPath $PreviousPath)) {
        return [PSCustomObject]@{
            Status       = "path not found"
            Exists       = $false
            Loaded       = $false
            ResolvedPath = ""
            Rows         = @()
        }
    }

    $ResolvedPath = (Resolve-Path -LiteralPath $PreviousPath).Path
    $Rows = @(Import-Csv -LiteralPath $ResolvedPath)

    return [PSCustomObject]@{
        Status       = "loaded"
        Exists       = $true
        Loaded       = $true
        ResolvedPath = $ResolvedPath
        Rows         = $Rows
    }
}

function Write-LowGapMetricLine {
    param(
        [string]$OutputPath,
        [string]$Label,
        [object]$Value
    )

    Add-Utf8Text -Path $OutputPath -Text "$Label`: $(Format-InvariantDouble -Value $Value)`r`n"
}

function Write-LowGapBaselineNotes {
    param(
        [string]$OutputPath,
        [string]$Title,
        [string]$BaselineName,
        [object]$CurrentRow,
        [object]$PreviousRow,
        [string]$PreviousCsvStatus,
        [double]$Threshold
    )

    Add-Utf8Text -Path $OutputPath -Text "`r`n$Title`r`n"
    Add-Utf8Text -Path $OutputPath -Text ("-" * $Title.Length)
    Add-Utf8Text -Path $OutputPath -Text "`r`n"

    if ($null -eq $CurrentRow) {
        Add-Utf8Text -Path $OutputPath -Text "current row: missing for baseline $BaselineName`r`n"
        Add-Utf8Text -Path $OutputPath -Text "classification: unavailable`r`n"
        return
    }

    $CurrentMeanTiming = Get-LowGapMetric -Row $CurrentRow -FieldName "low_mean_timing_ratio"
    $CurrentWeightedCandidate = Get-LowGapMetric -Row $CurrentRow -FieldName "low_weighted_candidate_ratio"
    $CurrentWeightedAvoidance = Get-LowGapMetric -Row $CurrentRow -FieldName "low_weighted_reconstruction_avoidance_ratio"

    Write-LowGapMetricLine -OutputPath $OutputPath -Label "current low mean timing ratio" -Value $CurrentMeanTiming
    Write-LowGapMetricLine -OutputPath $OutputPath -Label "current low weighted candidate ratio" -Value $CurrentWeightedCandidate
    Write-LowGapMetricLine -OutputPath $OutputPath -Label "current low weighted avoidance ratio" -Value $CurrentWeightedAvoidance

    if ($PreviousCsvStatus -ne "loaded") {
        Add-Utf8Text -Path $OutputPath -Text "previous CSV status: $PreviousCsvStatus`r`n"
        Add-Utf8Text -Path $OutputPath -Text "classification: unavailable`r`n"
        return
    }

    if ($null -eq $PreviousRow) {
        Add-Utf8Text -Path $OutputPath -Text "previous CSV status: loaded`r`n"
        Add-Utf8Text -Path $OutputPath -Text "previous row: missing for baseline $BaselineName`r`n"
        Add-Utf8Text -Path $OutputPath -Text "classification: unavailable`r`n"
        return
    }

    $PreviousMeanTiming = Get-LowGapMetric -Row $PreviousRow -FieldName "low_mean_timing_ratio"
    $PreviousWeightedCandidate = Get-LowGapMetric -Row $PreviousRow -FieldName "low_weighted_candidate_ratio"
    $PreviousWeightedAvoidance = Get-LowGapMetric -Row $PreviousRow -FieldName "low_weighted_reconstruction_avoidance_ratio"

    Write-LowGapMetricLine -OutputPath $OutputPath -Label "previous low mean timing ratio" -Value $PreviousMeanTiming
    Write-LowGapMetricLine -OutputPath $OutputPath -Label "previous low weighted candidate ratio" -Value $PreviousWeightedCandidate
    Write-LowGapMetricLine -OutputPath $OutputPath -Label "previous low weighted avoidance ratio" -Value $PreviousWeightedAvoidance

    if ($null -eq $CurrentMeanTiming -or $null -eq $PreviousMeanTiming) {
        Add-Utf8Text -Path $OutputPath -Text "comparison status: timing metric missing`r`n"
        Add-Utf8Text -Path $OutputPath -Text "classification: unavailable`r`n"
        return
    }

    $TimingDelta = $CurrentMeanTiming - $PreviousMeanTiming
    $Classification = Get-LowGapClassification -Delta $TimingDelta -Threshold $Threshold

    Add-Utf8Text -Path $OutputPath -Text "low mean timing delta: $(Format-InvariantDouble -Value $TimingDelta)`r`n"

    if ($null -ne $CurrentWeightedCandidate -and $null -ne $PreviousWeightedCandidate) {
        $CandidateDelta = $CurrentWeightedCandidate - $PreviousWeightedCandidate
        Add-Utf8Text -Path $OutputPath -Text "low weighted candidate delta: $(Format-InvariantDouble -Value $CandidateDelta)`r`n"
    }
    else {
        Add-Utf8Text -Path $OutputPath -Text "low weighted candidate delta: unavailable`r`n"
    }

    if ($null -ne $CurrentWeightedAvoidance -and $null -ne $PreviousWeightedAvoidance) {
        $AvoidanceDelta = $CurrentWeightedAvoidance - $PreviousWeightedAvoidance
        Add-Utf8Text -Path $OutputPath -Text "low weighted avoidance delta: $(Format-InvariantDouble -Value $AvoidanceDelta)`r`n"
    }
    else {
        Add-Utf8Text -Path $OutputPath -Text "low weighted avoidance delta: unavailable`r`n"
    }

    Add-Utf8Text -Path $OutputPath -Text "comparison status: performed`r`n"
    Add-Utf8Text -Path $OutputPath -Text "classification: $Classification`r`n"
}

function Write-LowGapRegressionNotes {
    param(
        [string]$CurrentPath,
        [string]$PreviousPath,
        [string]$OutputPath,
        [double]$Threshold
    )

    Set-Utf8Text -Path $OutputPath -Text "Low-selectivity gap regression notes`r`n"
    Add-Utf8Text -Path $OutputPath -Text "===================================`r`n`r`n"
    Add-Utf8Text -Path $OutputPath -Text "Current CSV: $CurrentPath`r`n"

    $PreviousCsvLoad = Get-PreviousCsvLoadStatus -PreviousPath $PreviousPath

    if ([string]::IsNullOrWhiteSpace($PreviousPath)) {
        Add-Utf8Text -Path $OutputPath -Text "Previous CSV: not provided`r`n"
    }
    else {
        Add-Utf8Text -Path $OutputPath -Text "Previous CSV: $PreviousPath`r`n"
    }

    Add-Utf8Text -Path $OutputPath -Text "Previous CSV status: $($PreviousCsvLoad.Status)`r`n"

    if ($PreviousCsvLoad.Loaded) {
        Add-Utf8Text -Path $OutputPath -Text "Previous CSV resolved path: $($PreviousCsvLoad.ResolvedPath)`r`n"
        Add-Utf8Text -Path $OutputPath -Text "Previous CSV rows loaded: $($PreviousCsvLoad.Rows.Count)`r`n"
    }

    Add-Utf8Text -Path $OutputPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $Threshold)`r`n"
    Add-Utf8Text -Path $OutputPath -Text "Timing ratio meaning: baseline elapsed / FSE elapsed; higher is better for FSE.`r`n"

    if (!(Test-Path -LiteralPath $CurrentPath)) {
        Add-Utf8Text -Path $OutputPath -Text "`r`nCurrent low-selectivity gap CSV status: path not found`r`n"
        Add-Utf8Text -Path $OutputPath -Text "classification: unavailable`r`n"
        return
    }

    $ResolvedCurrentPath = (Resolve-Path -LiteralPath $CurrentPath).Path
    $CurrentRows = @(Import-Csv -LiteralPath $ResolvedCurrentPath)

    Add-Utf8Text -Path $OutputPath -Text "Current CSV status: loaded`r`n"
    Add-Utf8Text -Path $OutputPath -Text "Current CSV resolved path: $ResolvedCurrentPath`r`n"
    Add-Utf8Text -Path $OutputPath -Text "Current CSV rows loaded: $($CurrentRows.Count)`r`n"

    $CurrentKdTree = Get-LowGapRow -Rows $CurrentRows -BaselineName "kd_tree"
    $PreviousKdTree = Get-LowGapRow -Rows $PreviousCsvLoad.Rows -BaselineName "kd_tree"

    $CurrentRTree = Get-LowGapRow -Rows $CurrentRows -BaselineName "r_tree"
    $PreviousRTree = Get-LowGapRow -Rows $PreviousCsvLoad.Rows -BaselineName "r_tree"

    Write-LowGapBaselineNotes `
        -OutputPath $OutputPath `
        -Title "KD-tree low-selectivity gap" `
        -BaselineName "kd_tree" `
        -CurrentRow $CurrentKdTree `
        -PreviousRow $PreviousKdTree `
        -PreviousCsvStatus $PreviousCsvLoad.Status `
        -Threshold $Threshold

    Write-LowGapBaselineNotes `
        -OutputPath $OutputPath `
        -Title "R-tree low-selectivity reference" `
        -BaselineName "r_tree" `
        -CurrentRow $CurrentRTree `
        -PreviousRow $PreviousRTree `
        -PreviousCsvStatus $PreviousCsvLoad.Status `
        -Threshold $Threshold

    Add-Utf8Text -Path $OutputPath -Text "`r`nDecision guidance`r`n"
    Add-Utf8Text -Path $OutputPath -Text "-----------------`r`n"
    Add-Utf8Text -Path $OutputPath -Text "Keep reporting-only commits when benchmark outputs and CSVs agree.`r`n"
    Add-Utf8Text -Path $OutputPath -Text "Treat KD-tree low-selectivity regressions outside the threshold as review signals, not automatic reverts.`r`n"
    Add-Utf8Text -Path $OutputPath -Text "Use workload CSV rows to identify which low-selectivity workload moved.`r`n"
}

$BaseBenchmarkArgs = @(
    "run",
    "--quiet",
    "--release",
    "--",
    "--dataset",
    $Dataset,
    "--all-baselines",
    "--iterations",
    $Iterations.ToString(),
    "--max-depth",
    $EffectiveMaxDepthText
)

Invoke-BenchmarkCommand `
    -OutputPath $TextOutputPath `
    -Title "Default $Dataset dataset policy:" `
    -CargoArguments $BaseBenchmarkArgs `
    -First

Invoke-BenchmarkCommand `
    -OutputPath $TextOutputPath `
    -Title "Explicit 8/8 Run:" `
    -CargoArguments ($BaseBenchmarkArgs + @(
        "--target-leaf-size",
        "8",
        "--max-leaf-size",
        "8"
    ))

Invoke-BenchmarkCommand `
    -OutputPath $TextOutputPath `
    -Title "Explicit 4/8 run:" `
    -CargoArguments ($BaseBenchmarkArgs + @(
        "--target-leaf-size",
        "4",
        "--max-leaf-size",
        "8"
    ))

Invoke-BenchmarkCommand `
    -OutputPath $TextOutputPath `
    -Title "CSV check:" `
    -CargoArguments ($BaseBenchmarkArgs + @(
        "--csv-summary",
        $SummaryCsvPath,
        "--csv-workloads",
        $WorkloadsCsvPath,
        "--csv-low-selectivity-gap",
        $LowSelectivityGapCsvPath
    ))

if (!(Test-Path -LiteralPath $CountOnlyWorkloadSummaryScript)) {
    throw "required script was not found: $CountOnlyWorkloadSummaryScript"
}

$CountOnlyWorkloadSummaryArguments = @{
    InputWorkloadsCsv = $WorkloadsCsvPath
    OutputCsv         = $CountOnlyWorkloadSummaryCsvPath
    OutputNotesPath   = $CountOnlyWorkloadSummaryNotesPath
}

& $CountOnlyWorkloadSummaryScript @CountOnlyWorkloadSummaryArguments

if ($LASTEXITCODE -ne 0) {
    throw "count-only workload summary failed with exit code $LASTEXITCODE"
}

Write-LowGapRegressionNotes `
    -CurrentPath $LowSelectivityGapCsvPath `
    -PreviousPath $PreviousLowSelectivityGapCsv `
    -OutputPath $LowGapRegressionNotesPath `
    -Threshold $LowGapNoiseThreshold

Invoke-BenchmarkCommand `
    -OutputPath $DebugOutputPath `
    -Title "Default $Dataset dataset debug report:" `
    -CargoArguments ($BaseBenchmarkArgs + @(
        "--debug-report"
    )) `
    -First

if (!(Test-Path -LiteralPath $MaterializationModeSummaryScript)) {
    throw "required script was not found: $MaterializationModeSummaryScript"
}

$MaterializationModeSummaryArguments = @{
    DebugOutputPath = $DebugOutputPath
    OutputCsv       = $MaterializationModeSummaryCsvPath
    OutputNotesPath = $MaterializationModeSummaryNotesPath
    Dataset         = $Dataset
    MaxDepth        = $EffectiveMaxDepthText
}

& $MaterializationModeSummaryScript @MaterializationModeSummaryArguments

if ($LASTEXITCODE -ne 0) {
    throw "materialization mode summary failed with exit code $LASTEXITCODE"
}

if (!(Test-Path -LiteralPath $ExistenceTimingSummaryScript)) {
    throw "required script was not found: $ExistenceTimingSummaryScript"
}

$ExistenceTimingSummaryArguments = @{
    DebugOutputPath = $DebugOutputPath
    OutputCsv       = $ExistenceTimingSummaryCsvPath
    OutputNotesPath = $ExistenceTimingSummaryNotesPath
    Dataset         = $Dataset
    MaxDepth        = $EffectiveMaxDepthText
}

& $ExistenceTimingSummaryScript @ExistenceTimingSummaryArguments

if ($LASTEXITCODE -ne 0) {
    throw "existence timing summary failed with exit code $LASTEXITCODE"
}

if (!(Test-Path -LiteralPath $TypedIndexedComparisonSummaryScript)) {
    throw "required script was not found: $TypedIndexedComparisonSummaryScript"
}

$TypedIndexedComparisonSummaryArguments = @{
    DebugOutputPath = $DebugOutputPath
    OutputCsv       = $TypedIndexedComparisonSummaryCsvPath
    OutputNotesPath = $TypedIndexedComparisonSummaryNotesPath
    Dataset         = $Dataset
    MaxDepth        = $EffectiveMaxDepthText
}

& $TypedIndexedComparisonSummaryScript @TypedIndexedComparisonSummaryArguments

if ($LASTEXITCODE -ne 0) {
    throw "typed indexed comparison summary failed with exit code $LASTEXITCODE"
}

Add-Utf8Text -Path $TextOutputPath -Text "`r`n---`r`n`r`nArtifacts:`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Dataset:                        $Dataset`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Max depth:                      $EffectiveMaxDepth`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Benchmark output:               $TextOutputPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Debug output:                   $DebugOutputPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Summary CSV:                    $SummaryCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Workloads CSV:                  $WorkloadsCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Count-only workload summary:    $CountOnlyWorkloadSummaryCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Count-only workload notes:      $CountOnlyWorkloadSummaryNotesPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Materialization summary CSV:    $MaterializationModeSummaryCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Materialization summary notes:  $MaterializationModeSummaryNotesPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Existence timing summary CSV:   $ExistenceTimingSummaryCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Existence timing notes:         $ExistenceTimingSummaryNotesPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Typed indexed summary CSV:      $TypedIndexedComparisonSummaryCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Typed indexed notes:            $TypedIndexedComparisonSummaryNotesPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Low-selectivity gap CSV:        $LowSelectivityGapCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Low-gap regression notes:       $LowGapRegressionNotesPath`r`n"

Add-Utf8Text -Path $DebugOutputPath -Text "`r`n---`r`n`r`nArtifacts:`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Dataset:                        $Dataset`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Max depth:                      $EffectiveMaxDepth`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Benchmark output:               $TextOutputPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Debug output:                   $DebugOutputPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Summary CSV:                    $SummaryCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Workloads CSV:                  $WorkloadsCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Count-only workload summary:    $CountOnlyWorkloadSummaryCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Count-only workload notes:      $CountOnlyWorkloadSummaryNotesPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Materialization summary CSV:    $MaterializationModeSummaryCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Materialization summary notes:  $MaterializationModeSummaryNotesPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Existence timing summary CSV:   $ExistenceTimingSummaryCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Existence timing notes:         $ExistenceTimingSummaryNotesPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Typed indexed summary CSV:      $TypedIndexedComparisonSummaryCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Typed indexed notes:            $TypedIndexedComparisonSummaryNotesPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Low-selectivity gap CSV:        $LowSelectivityGapCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Low-gap regression notes:       $LowGapRegressionNotesPath`r`n"

Write-Host ""
Write-Host "Benchmark artifacts written:"
Write-Host "  Dataset: $Dataset"
Write-Host "  Max depth: $EffectiveMaxDepth"
Write-Host "  $TextOutputPath"
Write-Host "  $DebugOutputPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $WorkloadsCsvPath"
Write-Host "  $CountOnlyWorkloadSummaryCsvPath"
Write-Host "  $CountOnlyWorkloadSummaryNotesPath"
Write-Host "  $MaterializationModeSummaryCsvPath"
Write-Host "  $MaterializationModeSummaryNotesPath"
Write-Host "  $ExistenceTimingSummaryCsvPath"
Write-Host "  $ExistenceTimingSummaryNotesPath"
Write-Host "  $TypedIndexedComparisonSummaryCsvPath"
Write-Host "  $TypedIndexedComparisonSummaryNotesPath"
Write-Host "  $LowSelectivityGapCsvPath"
Write-Host "  $LowGapRegressionNotesPath"
