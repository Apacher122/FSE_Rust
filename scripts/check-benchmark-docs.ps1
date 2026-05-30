$ErrorActionPreference = "Stop"

$TargetScript = Join-Path $PSScriptRoot "benchmark\check\check-benchmark-docs.ps1"

if (!(Test-Path -LiteralPath $TargetScript)) {
  throw "benchmark workflow script was not found: $TargetScript"
}

& $TargetScript @args
