param(
  [string]$DebugOutputPath,
  [string]$OutputCsv,
  [string]$OutputNotesPath = "",
  [string]$Dataset = "",
  [string]$MaxDepth = ""
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$BenchmarkCsvLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-csv.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

. $BenchmarkCsvLibrary

if ([string]::IsNullOrWhiteSpace($DebugOutputPath)) {
  throw "-DebugOutputPath cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($OutputCsv)) {
  throw "-OutputCsv cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($OutputNotesPath)) {
  $OutputNotesPath = [System.IO.Path]::ChangeExtension($OutputCsv, ".txt")
}

if (!(Test-Path -LiteralPath $DebugOutputPath)) {
  throw "debug output was not found: $DebugOutputPath"
}

$ResolvedDebugOutputPath = (Resolve-Path -LiteralPath $DebugOutputPath).Path
$Lines = [System.IO.File]::ReadAllLines($ResolvedDebugOutputPath)

if ([string]::IsNullOrWhiteSpace($Dataset)) {
  $Dataset = "unknown"
}

if ([string]::IsNullOrWhiteSpace($MaxDepth)) {
  $MaxDepth = "unknown"
}

foreach ($Line in $Lines) {
  $Trimmed = $Line.Trim()

  if ($Dataset -eq "unknown" -and $Trimmed.StartsWith("Dataset:")) {
    $Dataset = $Trimmed.Substring("Dataset:".Length).Trim()
  }

  if ($MaxDepth -eq "unknown" -and $Trimmed.StartsWith("Max depth:")) {
    $MaxDepth = $Trimmed.Substring("Max depth:".Length).Trim()
  }
}

$SectionIndex = -1

for ($Index = 0; $Index -lt $Lines.Count; $Index++) {
  if ($Lines[$Index].Trim() -eq "Workload typed indexed comparison summary") {
    $SectionIndex = $Index
    break
  }
}

if ($SectionIndex -lt 0) {
  throw "debug output does not contain a workload typed indexed comparison summary section"
}

$HeaderIndex = -1

for ($Index = $SectionIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Trimmed = $Lines[$Index].Trim()

  if ($Trimmed.StartsWith("workload | matched | typed scan | indexed typed | timing ratio | candidate ratio | planner strategy | selectivity bucket | planner risk | record avoidance | agreement")) {
    $HeaderIndex = $Index
    break
  }
}

if ($HeaderIndex -lt 0) {
  throw "workload typed indexed comparison summary header was not found"
}

$Rows = New-Object System.Collections.Generic.List[object[]]

for ($Index = $HeaderIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Line = $Lines[$Index]
  $Trimmed = $Line.Trim()

  if ([string]::IsNullOrWhiteSpace($Trimmed)) {
    break
  }

  if (!$Trimmed.Contains("|")) {
    break
  }

  $Columns = @($Trimmed.Split("|") | ForEach-Object { $_.Trim() })

  if ($Columns.Count -ne 11) {
    throw "unexpected typed indexed comparison summary column count at line $($Index + 1): $($Columns.Count)"
  }

  $Rows.Add(@(
      $Dataset,
      $MaxDepth,
      $Columns[0],
      $Columns[1],
      $Columns[2],
      $Columns[3],
      $Columns[4],
      $Columns[5],
      $Columns[6],
      $Columns[7],
      $Columns[8],
      $Columns[9],
      $Columns[10]
    )) | Out-Null
}

if ($Rows.Count -eq 0) {
  throw "workload typed indexed comparison summary contained no workload rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "typed_scan_elapsed",
  "indexed_typed_elapsed",
  "timing_ratio",
  "candidate_ratio",
  "planner_strategy",
  "selectivity_bucket",
  "planner_risk",
  "record_avoidance_ratio",
  "agreement"
)

Write-CsvDocument `
  -Path $OutputCsv `
  -Header $Header `
  -Rows $Rows

$FailedRows = @($Rows | Where-Object { $_[12] -ne "pass" })

Set-Utf8Text -Path $OutputNotesPath -Text "Typed indexed comparison summary notes`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "======================================`r`n`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Debug output: $ResolvedDebugOutputPath`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Output CSV: $OutputCsv`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Max depth: $MaxDepth`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Workload rows: $($Rows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Agreement failures: $($FailedRows.Count)`r`n"

if ($FailedRows.Count -gt 0) {
  Add-Utf8Text -Path $OutputNotesPath -Text "`r`nFailed workload rows`r`n"
  Add-Utf8Text -Path $OutputNotesPath -Text "--------------------`r`n"

  foreach ($FailedRow in $FailedRows) {
    Add-Utf8Text -Path $OutputNotesPath -Text "$($FailedRow[2]): $($FailedRow[12])`r`n"
  }
}

Write-Host "Typed indexed comparison summary written:"
Write-Host "  $OutputCsv"
Write-Host "  $OutputNotesPath"
