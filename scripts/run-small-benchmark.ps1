param(
    [string]$Label = "local",
    [int]$Iterations = 10000,
    [string]$OutputDir = "benchmark_artifacts"
)

$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TextOutputPath = Join-Path $OutputDir "benchmark-output-$Label.txt"
$DebugOutputPath = Join-Path $OutputDir "debug-output-$Label.txt"
$SummaryCsvPath = Join-Path $OutputDir "summary-$Label.csv"
$WorkloadsCsvPath = Join-Path $OutputDir "workloads-$Label.csv"

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)

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
        $WorkloadsCsvPath
    ))

Invoke-BenchmarkCommand `
    -OutputPath $DebugOutputPath `
    -Title "Default 8/8 debug report:" `
    -CargoArguments ($BaseBenchmarkArgs + @(
        "--debug-report"
    )) `
    -First

Add-Utf8Text -Path $TextOutputPath -Text "`r`n---`r`n`r`nArtifacts:`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Benchmark output: $TextOutputPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Debug output:     $DebugOutputPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Summary CSV:      $SummaryCsvPath`r`n"
Add-Utf8Text -Path $TextOutputPath -Text "  Workloads CSV:    $WorkloadsCsvPath`r`n"

Add-Utf8Text -Path $DebugOutputPath -Text "`r`n---`r`n`r`nArtifacts:`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Benchmark output: $TextOutputPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Debug output:     $DebugOutputPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Summary CSV:      $SummaryCsvPath`r`n"
Add-Utf8Text -Path $DebugOutputPath -Text "  Workloads CSV:    $WorkloadsCsvPath`r`n"

Write-Host ""
Write-Host "Benchmark artifacts written:"
Write-Host "  $TextOutputPath"
Write-Host "  $DebugOutputPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $WorkloadsCsvPath"