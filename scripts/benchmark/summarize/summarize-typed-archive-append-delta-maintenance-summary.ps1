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
  if ($Lines[$Index].Trim() -eq "Workload typed archive append-delta maintenance timing summary") {
    $SectionIndex = $Index
    break
  }
}

if ($SectionIndex -lt 0) {
  throw "debug output does not contain a workload typed archive append-delta maintenance timing summary section"
}

$HeaderIndex = -1

for ($Index = $SectionIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Trimmed = $Lines[$Index].Trim()

  if ($Trimmed.StartsWith("workload | action | reason | status write required | base records | pending append | tombstones | resulting records | index before bytes | index after bytes | index byte delta | append before bytes | append after bytes | append byte delta | tombstone before bytes | tombstone after bytes | tombstone byte delta | logical before bytes | logical after bytes | logical byte delta | matched after maintenance | maintenance | agreement")) {
    $HeaderIndex = $Index
    break
  }
}

if ($HeaderIndex -lt 0) {
  throw "workload typed archive append-delta maintenance timing summary header was not found"
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

  if ($Columns.Count -ne 23) {
    throw "unexpected typed archive append-delta maintenance timing summary column count at line $($Index + 1): $($Columns.Count)"
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
      $Columns[13],
      $Columns[14],
      $Columns[15],
      $Columns[16],
      $Columns[17],
      $Columns[18],
      $Columns[19],
      $Columns[20],
      $Columns[21],
      $Columns[22]
    )) | Out-Null
}

if ($Rows.Count -eq 0) {
  throw "workload typed archive append-delta maintenance timing summary contained no workload rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "maintenance_action",
  "maintenance_reason",
  "maintenance_status_requires_archive_write",
  "base_record_count",
  "pending_append_record_count",
  "tombstone_count",
  "resulting_record_count",
  "index_bytes_before_maintenance",
  "index_bytes_after_maintenance",
  "index_byte_delta",
  "append_bytes_before_maintenance",
  "append_bytes_after_maintenance",
  "append_byte_delta",
  "tombstone_bytes_before_maintenance",
  "tombstone_bytes_after_maintenance",
  "tombstone_byte_delta",
  "logical_archive_bytes_before_maintenance",
  "logical_archive_bytes_after_maintenance",
  "logical_archive_byte_delta",
  "matched_records_after_maintenance",
  "maintenance_elapsed",
  "agreement"
)

Write-CsvDocument `
  -Path $OutputCsv `
  -Header $Header `
  -Rows $Rows

$FailedRows = @($Rows | Where-Object { $_[24] -ne "pass" })
$TotalMatchedRecordsAfterMaintenance = 0

foreach ($Row in $Rows) {
  $TotalMatchedRecordsAfterMaintenance += Convert-ToInvariantIntOrZero -Value $Row[22]
}

$MaintenanceActions = @($Rows | ForEach-Object { $_[3] } | Sort-Object -Unique)
$MaintenanceReasons = @($Rows | ForEach-Object { $_[4] } | Sort-Object -Unique)
$MaintenanceStatusWriteRequiredValues = @($Rows | ForEach-Object { $_[5] } | Sort-Object -Unique)
$BaseRecordCounts = @($Rows | ForEach-Object { $_[6] } | Sort-Object -Unique)
$PendingAppendRecordCounts = @($Rows | ForEach-Object { $_[7] } | Sort-Object -Unique)
$TombstoneCounts = @($Rows | ForEach-Object { $_[8] } | Sort-Object -Unique)
$ResultingRecordCounts = @($Rows | ForEach-Object { $_[9] } | Sort-Object -Unique)
$IndexByteDeltaValues = @($Rows | ForEach-Object { $_[12] } | Sort-Object -Unique)
$AppendByteDeltaValues = @($Rows | ForEach-Object { $_[15] } | Sort-Object -Unique)
$TombstoneByteDeltaValues = @($Rows | ForEach-Object { $_[18] } | Sort-Object -Unique)
$LogicalArchiveByteDeltaValues = @($Rows | ForEach-Object { $_[21] } | Sort-Object -Unique)

Set-Utf8Text -Path $OutputNotesPath -Text "Typed archive append-delta maintenance timing summary notes`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "==============================================================`r`n`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Debug output: $ResolvedDebugOutputPath`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Output CSV: $OutputCsv`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Max depth: $MaxDepth`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Workload rows: $($Rows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Agreement failures: $($FailedRows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total matched records after maintenance: $TotalMatchedRecordsAfterMaintenance`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Maintenance actions: $($MaintenanceActions -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Maintenance reasons: $($MaintenanceReasons -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Maintenance status write required values: $($MaintenanceStatusWriteRequiredValues -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Base record counts: $($BaseRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Pending append record counts: $($PendingAppendRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Tombstone counts: $($TombstoneCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Resulting record counts: $($ResultingRecordCounts -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Index byte delta values: $($IndexByteDeltaValues -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Append byte delta values: $($AppendByteDeltaValues -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Tombstone byte delta values: $($TombstoneByteDeltaValues -join ', ')`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Logical archive byte delta values: $($LogicalArchiveByteDeltaValues -join ', ')`r`n"

if ($FailedRows.Count -gt 0) {
  Add-Utf8Text -Path $OutputNotesPath -Text "`r`nFailed workload rows`r`n"
  Add-Utf8Text -Path $OutputNotesPath -Text "--------------------`r`n"

  foreach ($FailedRow in $FailedRows) {
    Add-Utf8Text -Path $OutputNotesPath -Text "$($FailedRow[2]): $($FailedRow[24])`r`n"
  }
}

Write-Host "Typed archive append-delta maintenance timing summary written:"
Write-Host "  $OutputCsv"
Write-Host "  $OutputNotesPath"
