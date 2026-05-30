param(
  [string]$Label = "leaf-policy-trials",
  [ValidateSet("small", "large")]
  [string]$Dataset = "small",
  [int]$Trials = 3,
  [int]$Iterations = 10000,
  [int[]]$TargetLeafSizes = @(4, 8, 16),
  [int[]]$MaxLeafSizes = @(8, 16, 32),
  [string]$OutputDir = "benchmark_artifacts",
  [string]$CargoCommand = "cargo run --release"
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$LegacyScript = Join-Path $ScriptsRoot "legacy/run-leaf-policy-trials.ps1"

if (!(Test-Path -LiteralPath $LegacyScript)) {
  throw "legacy leaf policy runner was not found: $LegacyScript"
}

Write-Host "The leaf-policy trial runner has been moved to:"
Write-Host "  $LegacyScript"
Write-Host ""
Write-Host "This wrapper exists for compatibility with older notes and commands."
Write-Host ""

& $LegacyScript `
  -Label $Label `
  -Dataset $Dataset `
  -Trials $Trials `
  -Iterations $Iterations `
  -TargetLeafSizes $TargetLeafSizes `
  -MaxLeafSizes $MaxLeafSizes `
  -OutputDir $OutputDir `
  -CargoCommand $CargoCommand