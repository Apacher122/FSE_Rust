$ErrorActionPreference = "Stop"

$TargetScript = Join-Path $PSScriptRoot "benchmark\compare\compare-low-gap-workload-summary.ps1"

if (!(Test-Path -LiteralPath $TargetScript)) {
  throw "benchmark workflow script was not found: $TargetScript"
}

& $TargetScript @args
