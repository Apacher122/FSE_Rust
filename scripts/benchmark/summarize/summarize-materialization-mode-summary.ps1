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
  if ($Lines[$Index].Trim() -eq "Workload materialization mode summary") {
    $SectionIndex = $Index
    break
  }
}

if ($SectionIndex -lt 0) {
  throw "debug output does not contain a workload materialization mode summary section"
}

$HeaderIndex = -1

for ($Index = $SectionIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Trimmed = $Lines[$Index].Trim()

  if ($Trimmed.StartsWith("workload | matched | fresh owned | reusable owned | reference | visitor | count-only")) {
    $HeaderIndex = $Index
    break
  }
}

if ($HeaderIndex -lt 0) {
  throw "workload materialization summary header was not found"
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

  if ($Columns.Count -ne 12) {
    throw "unexpected materialization summary column count at line $($Index + 1): $($Columns.Count)"
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
      $Columns[10],
      $Columns[11]
    )) | Out-Null
}

if ($Rows.Count -eq 0) {
  throw "workload materialization mode summary contained no workload rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "fresh_owned_elapsed",
  "reusable_owned_elapsed",
  "reference_elapsed",
  "visitor_elapsed",
  "count_only_elapsed",
  "count_speedup",
  "reference_speedup",
  "visitor_speedup",
  "reusable_speedup",
  "agreement"
)

Write-CsvDocument `
  -Path $OutputCsv `
  -Header $Header `
  -Rows $Rows

$FailedRows = @($Rows | Where-Object { $_[13] -ne "pass" })

Set-Utf8Text -Path $OutputNotesPath -Text "Materialization mode summary notes`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "==================================`r`n`r`n"
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
    Add-Utf8Text -Path $OutputNotesPath -Text "$($FailedRow[2]): $($FailedRow[13])`r`n"
  }
}

Write-Host "Materialization mode summary written:"
Write-Host "  $OutputCsv"
Write-Host "  $OutputNotesPath"
