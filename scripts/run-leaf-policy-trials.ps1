$ErrorActionPreference = "Stop"

$TargetScript = Join-Path $PSScriptRoot "benchmark\run\run-leaf-policy-trials.ps1"

if (!(Test-Path -LiteralPath $TargetScript)) {
  throw "benchmark workflow script was not found: $TargetScript"
}

& $TargetScript @args
