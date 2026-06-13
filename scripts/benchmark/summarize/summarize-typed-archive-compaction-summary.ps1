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
  if ($Lines[$Index].Trim() -eq "Workload typed archive compaction timing summary") {
    $SectionIndex = $Index
    break
  }
}

if ($SectionIndex -lt 0) {
  throw "debug output does not contain a workload typed archive compaction timing summary section"
}

$HeaderIndex = -1

for ($Index = $SectionIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Trimmed = $Lines[$Index].Trim()

  if ($Trimmed.StartsWith("workload | base records | tombstones | removed records | retained records | index before bytes | index after bytes | index byte delta | tombstone before bytes | tombstone after bytes | tombstone byte delta | matched after compaction | compaction | agreement")) {
    $HeaderIndex = $Index
    break
  }
}

if ($HeaderIndex -lt 0) {
  throw "workload typed archive compaction timing summary header was not found"
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
    throw "unexpected typed archive compaction timing summary column count at line $($Index + 1): $($Columns.Count)"
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
  throw "workload typed archive compaction timing summary contained no workload rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "base_record_count",
  "tombstone_count",
  "removed_record_count",
  "retained_record_count",
  "index_bytes_before_compaction",
  "index_bytes_after_compaction",
  "index_byte_delta",
  "tombstone_bytes_before_compaction",
  "tombstone_bytes_after_compaction",
  "tombstone_byte_delta",
  "matched_records_after_compaction",
  "compaction_elapsed",
  "agreement"
)

Write-CsvDocument `
  -Path $OutputCsv `
  -Header $Header `
  -Rows $Rows

$FailedRows = @($Rows | Where-Object { $_[15] -ne "pass" })
$TotalMatchedRecordsAfterCompaction = 0

foreach ($Row in $Rows) {
  $TotalMatchedRecordsAfterCompaction += Convert-ToInvariantIntOrZero -Value $Row[13]
}

$BaseRecordCounts = @($Rows | ForEach-Object { $_[3] } | Sort-Object -Unique)
$TombstoneCounts = @($Rows | ForEach-Object { $_[4] } | Sort-Object -Unique)
$RemovedRecordCounts = @($Rows | ForEach-Object { $_[5] } | Sort-Object -Unique)
$RetainedRecordCounts = @($Rows | ForEach-Object { $_[6] } | Sort-Object -Unique)
$IndexByteDeltaValues = @($Rows | ForEach-Object { $_[9] } | Sort-Object -Unique)
$TombstoneByteDeltaValues = @($Rows | ForEach-Object { $_[12] } | Sort-Object -Unique)

Set-Utf8Text -Path $OutputNotesPath -Text "Typed archive compaction timing summary notes`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "=============================================`r`n`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Debug output: $ResolvedDebugOutputPath`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Output CSV: $OutputCsv`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Max depth: $MaxDepth`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Workload rows: $($Rows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Agreement failures: $($FailedRows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total matched records after compaction: $TotalMatchedRecordsAfterCompaction`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Base record counts: $($BaseRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Tombstone counts: $($TombstoneCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Removed record counts: $($RemovedRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Retained record counts: $($RetainedRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Index byte delta values: $($IndexByteDeltaValues -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Tombstone byte delta values: $($TombstoneByteDeltaValues -join ', ')`r`n"

if ($FailedRows.Count -gt 0) {
  Add-Utf8Text -Path $OutputNotesPath -Text "`r`nFailed workload rows`r`n"
  Add-Utf8Text -Path $OutputNotesPath -Text "--------------------`r`n"

  foreach ($FailedRow in $FailedRows) {
    Add-Utf8Text -Path $OutputNotesPath -Text "$($FailedRow[2]): $($FailedRow[15])`r`n"
  }
}

Write-Host "Typed archive compaction timing summary written:"
Write-Host "  $OutputCsv"
Write-Host "  $OutputNotesPath"
