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
  if ($Lines[$Index].Trim() -eq "Workload typed archive append-delta query timing summary") {
    $SectionIndex = $Index
    break
  }
}

if ($SectionIndex -lt 0) {
  throw "debug output does not contain a workload typed archive append-delta query timing summary section"
}

$HeaderIndex = -1

for ($Index = $SectionIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Trimmed = $Lines[$Index].Trim()

  if ($Trimmed.StartsWith("workload | base records | appended records | rebuilt records | append-delta matched | rebuilt matched | append-delta reconstructed | rebuilt reconstructed | append-delta candidate ratio | rebuilt candidate ratio | rebuilt/append-delta | rebuilt query | append-delta query | agreement")) {
    $HeaderIndex = $Index
    break
  }
}

if ($HeaderIndex -lt 0) {
  throw "workload typed archive append-delta query timing summary header was not found"
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
    throw "unexpected typed archive append-delta query timing summary column count at line $($Index + 1): $($Columns.Count)"
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
  throw "workload typed archive append-delta query timing summary contained no workload rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "base_record_count",
  "appended_record_count",
  "rebuilt_record_count",
  "append_delta_matched_records",
  "rebuilt_matched_records",
  "append_delta_reconstructed_records",
  "rebuilt_reconstructed_records",
  "append_delta_candidate_ratio",
  "rebuilt_candidate_ratio",
  "rebuilt_to_append_delta_ratio",
  "rebuilt_query_elapsed",
  "append_delta_query_elapsed",
  "agreement"
)

Write-CsvDocument `
  -Path $OutputCsv `
  -Header $Header `
  -Rows $Rows

$FailedRows = @($Rows | Where-Object { $_[15] -ne "pass" })
$TotalAppendDeltaMatchedRecords = 0
$TotalRebuiltMatchedRecords = 0
$TotalAppendDeltaReconstructedRecords = 0
$TotalRebuiltReconstructedRecords = 0

foreach ($Row in $Rows) {
  $TotalAppendDeltaMatchedRecords += Convert-ToInvariantIntOrZero -Value $Row[6]
  $TotalRebuiltMatchedRecords += Convert-ToInvariantIntOrZero -Value $Row[7]
  $TotalAppendDeltaReconstructedRecords += Convert-ToInvariantIntOrZero -Value $Row[8]
  $TotalRebuiltReconstructedRecords += Convert-ToInvariantIntOrZero -Value $Row[9]
}

$BaseRecordCounts = @($Rows | ForEach-Object { $_[3] } | Sort-Object -Unique)
$AppendedRecordCounts = @($Rows | ForEach-Object { $_[4] } | Sort-Object -Unique)
$RebuiltRecordCounts = @($Rows | ForEach-Object { $_[5] } | Sort-Object -Unique)
$AppendDeltaCandidateRatios = @($Rows | ForEach-Object { $_[10] } | Sort-Object -Unique)
$RebuiltCandidateRatios = @($Rows | ForEach-Object { $_[11] } | Sort-Object -Unique)

Set-Utf8Text -Path $OutputNotesPath -Text "Typed archive append-delta query timing summary notes`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "========================================================`r`n`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Debug output: $ResolvedDebugOutputPath`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Output CSV: $OutputCsv`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Max depth: $MaxDepth`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Workload rows: $($Rows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Agreement failures: $($FailedRows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total append-delta matched records: $TotalAppendDeltaMatchedRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total rebuilt matched records: $TotalRebuiltMatchedRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total append-delta reconstructed records: $TotalAppendDeltaReconstructedRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total rebuilt reconstructed records: $TotalRebuiltReconstructedRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Base record counts: $($BaseRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Appended record counts: $($AppendedRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Rebuilt record counts: $($RebuiltRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Append-delta candidate ratios: $($AppendDeltaCandidateRatios -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Rebuilt candidate ratios: $($RebuiltCandidateRatios -join ', ')`r`n"

if ($FailedRows.Count -gt 0) {
  Add-Utf8Text -Path $OutputNotesPath -Text "`r`nFailed workload rows`r`n"
  Add-Utf8Text -Path $OutputNotesPath -Text "--------------------`r`n"

  foreach ($FailedRow in $FailedRows) {
    Add-Utf8Text -Path $OutputNotesPath -Text "$($FailedRow[2]): $($FailedRow[15])`r`n"
  }
}

Write-Host "Typed archive append-delta query timing summary written:"
Write-Host "  $OutputCsv"
Write-Host "  $OutputNotesPath"
