$ErrorActionPreference = "Stop"

$TargetScript = Join-Path $PSScriptRoot "benchmark\run\run-small-benchmark.ps1"

if (!(Test-Path -LiteralPath $TargetScript)) {
    throw "benchmark workflow script was not found: $TargetScript"
}

& $TargetScript @args
