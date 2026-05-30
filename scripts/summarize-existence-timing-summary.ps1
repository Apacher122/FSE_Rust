param(
  [string]$DebugOutputPath,
  [string]$OutputCsv,
  [string]$OutputNotesPath = "",
  [string]$Dataset = "",
  [string]$MaxDepth = ""
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"

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
  if ($Lines[$Index].Trim() -eq "Workload exact existence timing summary") {
    $SectionIndex = $Index
    break
  }
}

if ($SectionIndex -lt 0) {
  throw "debug output does not contain a workload exact existence timing summary section"
}

$HeaderIndex = -1

for ($Index = $SectionIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Trimmed = $Lines[$Index].Trim()

  if ($Trimmed.StartsWith("workload | owned has match | count-only has match | existence has match | count-only candidate records")) {
    $HeaderIndex = $Index
    break
  }
}

if ($HeaderIndex -lt 0) {
  throw "workload exact existence timing summary header was not found"
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

  if ($Columns.Count -ne 14) {
    throw "unexpected existence timing summary column count at line $($Index + 1): $($Columns.Count)"
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
      $Columns[11],
      $Columns[12],
      $Columns[13]
    )) | Out-Null
}

if ($Rows.Count -eq 0) {
  throw "workload exact existence timing summary contained no workload rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "owned_has_match",
  "count_only_has_match",
  "existence_has_match",
  "count_only_candidate_records",
  "existence_inspected_records",
  "existence_skipped_records",
  "existence_inspection_ratio",
  "fresh_owned_elapsed",
  "count_only_elapsed",
  "existence_elapsed",
  "existence_speedup_vs_owned",
  "existence_speedup_vs_count_only",
  "agreement"
)

Write-CsvDocument `
  -Path $OutputCsv `
  -Header $Header `
  -Rows $Rows

$FailedRows = @($Rows | Where-Object { $_[15] -ne "pass" })
$TotalCandidateRecords = 0
$TotalInspectedRecords = 0
$TotalSkippedRecords = 0

foreach ($Row in $Rows) {
  $TotalCandidateRecords += Convert-ToInvariantIntOrZero -Value $Row[6]
  $TotalInspectedRecords += Convert-ToInvariantIntOrZero -Value $Row[7]
  $TotalSkippedRecords += Convert-ToInvariantIntOrZero -Value $Row[8]
}

$AggregateInspectionRatio = "0.000000"

if ($TotalCandidateRecords -gt 0) {
  $AggregateInspectionRatio = Format-InvariantDouble -Value ($TotalInspectedRecords / $TotalCandidateRecords)
}

Set-Utf8Text -Path $OutputNotesPath -Text "Existence timing summary notes`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "==============================`r`n`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Debug output: $ResolvedDebugOutputPath`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Output CSV: $OutputCsv`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Max depth: $MaxDepth`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Workload rows: $($Rows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Agreement failures: $($FailedRows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total count-only candidate records: $TotalCandidateRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total existence inspected records: $TotalInspectedRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total existence skipped records: $TotalSkippedRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Aggregate existence inspection ratio: $AggregateInspectionRatio`r`n"

if ($FailedRows.Count -gt 0) {
  Add-Utf8Text -Path $OutputNotesPath -Text "`r`nFailed workload rows`r`n"
  Add-Utf8Text -Path $OutputNotesPath -Text "--------------------`r`n"

  foreach ($FailedRow in $FailedRows) {
    Add-Utf8Text -Path $OutputNotesPath -Text "$($FailedRow[2]): $($FailedRow[15])`r`n"
  }
}

Write-Host "Existence timing summary written:"
Write-Host "  $OutputCsv"
Write-Host "  $OutputNotesPath"
