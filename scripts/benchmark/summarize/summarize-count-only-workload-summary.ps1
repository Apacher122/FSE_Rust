param(
  [string]$InputWorkloadsCsv,
  [string]$OutputCsv,
  [string]$OutputNotesPath = ""
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$BenchmarkCsvLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-csv.ps1"
$BenchmarkCountOnlyLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-count-only.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkCountOnlyLibrary)) {
  throw "benchmark count-only helper library was not found: $BenchmarkCountOnlyLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkCountOnlyLibrary

Invoke-CountOnlyWorkloadSummary `
  -InputWorkloadsCsv $InputWorkloadsCsv `
  -OutputCsv $OutputCsv `
  -OutputNotesPath $OutputNotesPath
