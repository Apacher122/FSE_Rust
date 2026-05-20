param(
    [string]$Label = "local",
    [int]$Iterations = 10000,
    [string]$OutputDir = "benchmark_artifacts",
    [string]$PreviousLowSelectivityGapCsv = "",
    [double]$LowGapNoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TextOutputPath = Join-Path $OutputDir "benchmark-output-$Label.txt"
$DebugOutputPath = Join-Path $OutputDir "debug-output-$Label.txt"
$SummaryCsvPath = Join-Path $OutputDir "summary-$Label.csv"
$WorkloadsCsvPath = Join-Path $OutputDir "workloads-$Label.csv"
$LowSelectivityGapCsvPath = Join-Path $OutputDir "low-selectivity-gap-$Label.csv"
$LowGapRegressionNotesPath = Join-Path $OutputDir "low-gap-regression-notes-$Label.txt"

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

function Convert-ToInvariantDouble {
    param(
        [string]$Value
    )

    return [double]::Parse($Value, $InvariantCulture)
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
    } catch {
        return "unavailable"
    }
}

function Get-LowGapRow {
    param(
        [object[]]$Rows,
        [string]$BaselineName
    )

    return $Rows | Where-Object { $_.baseline_name -eq $BaselineName } | Select-Object -First 1
}

function Get-LowGapMetric {
    param(
        [object]$Row,
        [string]$FieldName
    )

    if ($null -eq $Row) {
        return $null
    }

    $RawValue = $Row.$FieldName

    if ([string]::IsNullOrWhiteSpace($RawValue)) {
        return $null
    }

    return Convert-ToInvariantDouble -Value $RawValue
}

function Get-LowGapClassification {
    param(
        [double]$Delta,
        [double]$Threshold
    )

    if ($Delta -ge $Threshold) {
        return "improved"
    }

    if ($Delta -le (-1.0 * $Threshold)) {
        return "regressed"
    }

    return "stable/noise"
}

function Get-PreviousCsvLoadStatus {
    param(
        [string]$PreviousPath
    )

    if ([string]::IsNullOrWhiteSpace($PreviousPath)) {
        return [PSCustomObject]@{
            Status = "not provided"
            Exists = $false
            Loaded = $false
            ResolvedPath = ""
            Rows = @()
        }
    }

    if (!(Test-Path -LiteralPath $PreviousPath)) {
        return [PSCustomObject]@{
            Status = "path not found"
            Exists = $false
            Loaded = $false
            ResolvedPath = ""
            Rows = @()
        }
    }

    $ResolvedPath = (Resolve-Path -LiteralPath $PreviousPath).Path
    $Rows = @(Import-Csv -LiteralPath $ResolvedPath)

    return [PSCustomObject]@{
        Status = "loaded"
        Exists = $true
        Loaded = $true
        ResolvedPath = $ResolvedPath
        Rows = $Rows
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
    } else {
        Add-Utf8Text -Path $OutputPath -Text "low weighted candidate delta: unavailable`r`n"
    }

    if ($null -ne $CurrentWeightedAvoidance -and $null -ne $PreviousWeightedAvoidance) {
        $AvoidanceDelta = $CurrentWeightedAvoidance - $PreviousWeightedAvoidance
        Add-Utf8Text -Path $OutputPath -Text "low weighted avoidance delta: $(Format-InvariantDouble -Value $AvoidanceDelta)`r`n"
    } else {
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
    } else {
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
    "small",
    "--all-baselines",
    "--iterations",
    $Iterations.ToString()
)

Invoke-BenchmarkCommand `
    -OutputPath $TextOutputPath `
    -Title "Default 8/8 policy:" `
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

Write-LowGapRegressionNotes `
    -CurrentPath $LowSelectivityGapCsvPath `
    -PreviousPath $PreviousLowSelectivityGapCsv `
    -OutputPath $LowGapRegressionNotesPath `
    -Threshold $LowGapNoiseThreshold

Invoke-BenchmarkCommand `
    -OutputPath $DebugOutputPath `
    -Title "Default 8/8 debug report:" `
    -CargoArguments ($BaseBenchmarkArgs + @(
        "--debug-report"
    )) `
    -First

Add-Utf8Text -Path $TextOutputPath -Text "`r`n---`r`n`r`nArtifacts:`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Benchmark output:               $TextOutputPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Debug output:                   $DebugOutputPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Summary CSV:                    $SummaryCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Workloads CSV:                  $WorkloadsCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Low-selectivity gap CSV:        $LowSelectivityGapCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Low-gap regression notes:       $LowGapRegressionNotesPath`r`n"

Add-Utf8Text -Path $DebugOutputPath -Text "`r`n---`r`n`r`nArtifacts:`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Benchmark output:               $TextOutputPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Debug output:                   $DebugOutputPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Summary CSV:                    $SummaryCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Workloads CSV:                  $WorkloadsCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Low-selectivity gap CSV:        $LowSelectivityGapCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Low-gap regression notes:       $LowGapRegressionNotesPath`r`n"

Write-Host ""
Write-Host "Benchmark artifacts written:"
Write-Host "  $TextOutputPath"
Write-Host "  $DebugOutputPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $WorkloadsCsvPath"
Write-Host "  $LowSelectivityGapCsvPath"
Write-Host "  $LowGapRegressionNotesPath"