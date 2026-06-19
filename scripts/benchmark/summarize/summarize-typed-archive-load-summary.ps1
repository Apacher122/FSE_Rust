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
  if ($Lines[$Index].Trim() -eq "Workload typed archive load timing summary") {
    $SectionIndex = $Index
    break
  }
}

if ($SectionIndex -lt 0) {
  throw "debug output does not contain a workload typed archive load timing summary section"
}

$HeaderIndex = -1

for ($Index = $SectionIndex + 1; $Index -lt $Lines.Count; $Index++) {
  $Trimmed = $Lines[$Index].Trim()

  if ($Trimmed.StartsWith("workload | matched | in-memory | warm-loaded | cold-loaded | warm/in-memory | cold/in-memory | cold/warm | agreement")) {
    $HeaderIndex = $Index
    break
  }
}

if ($HeaderIndex -lt 0) {
  throw "workload typed archive load timing summary header was not found"
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

  if ($Columns.Count -ne 9) {
    throw "unexpected typed archive load timing summary column count at line $($Index + 1): $($Columns.Count)"
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
      $Columns[8]
    )) | Out-Null
}

if ($Rows.Count -eq 0) {
  throw "workload typed archive load timing summary contained no workload rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "in_memory_elapsed",
  "warm_loaded_elapsed",
  "cold_loaded_elapsed",
  "warm_loaded_to_in_memory_ratio",
  "cold_loaded_to_in_memory_ratio",
  "cold_loaded_to_warm_loaded_ratio",
  "agreement"
)

Write-CsvDocument `
  -Path $OutputCsv `
  -Header $Header `
  -Rows $Rows

$FailedRows = @($Rows | Where-Object { $_[10] -ne "pass" })
$TotalMatchedRecords = 0
$WarmLoadedRatios = New-Object System.Collections.Generic.List[double]
$ColdLoadedRatios = New-Object System.Collections.Generic.List[double]
$ColdToWarmRatios = New-Object System.Collections.Generic.List[double]

foreach ($Row in $Rows) {
  $TotalMatchedRecords += Convert-ToInvariantIntOrZero -Value $Row[3]

  $WarmLoadedRatio = Convert-ToInvariantDouble -Value $Row[7]
  $ColdLoadedRatio = Convert-ToInvariantDouble -Value $Row[8]
  $ColdToWarmRatio = Convert-ToInvariantDouble -Value $Row[9]

  if ($null -ne $WarmLoadedRatio) {
    $WarmLoadedRatios.Add($WarmLoadedRatio) | Out-Null
  }

  if ($null -ne $ColdLoadedRatio) {
    $ColdLoadedRatios.Add($ColdLoadedRatio) | Out-Null
  }

  if ($null -ne $ColdToWarmRatio) {
    $ColdToWarmRatios.Add($ColdToWarmRatio) | Out-Null
  }
}

$WarmLoadedRatioValues = @($WarmLoadedRatios.ToArray())
$ColdLoadedRatioValues = @($ColdLoadedRatios.ToArray())
$ColdToWarmRatioValues = @($ColdToWarmRatios.ToArray())
$MeanWarmLoadedRatio = Get-Mean -Values $WarmLoadedRatioValues
$MeanColdLoadedRatio = Get-Mean -Values $ColdLoadedRatioValues
$MeanColdToWarmRatio = Get-Mean -Values $ColdToWarmRatioValues

Set-Utf8Text -Path $OutputNotesPath -Text "Typed archive load timing summary notes`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "=======================================`r`n`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Debug output: $ResolvedDebugOutputPath`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Output CSV: $OutputCsv`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Max depth: $MaxDepth`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Workload rows: $($Rows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Agreement failures: $($FailedRows.Count)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Total matched records: $TotalMatchedRecords`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Mean warm-loaded/in-memory ratio: $(Format-InvariantDouble -Value $MeanWarmLoadedRatio)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Mean cold-loaded/in-memory ratio: $(Format-InvariantDouble -Value $MeanColdLoadedRatio)`r`n"
Add-Utf8Text -Path $OutputNotesPath -Text "Mean cold-loaded/warm-loaded ratio: $(Format-InvariantDouble -Value $MeanColdToWarmRatio)`r`n"

if ($FailedRows.Count -gt 0) {
  Add-Utf8Text -Path $OutputNotesPath -Text "`r`nFailed workload rows`r`n"
  Add-Utf8Text -Path $OutputNotesPath -Text "--------------------`r`n"

  foreach ($FailedRow in $FailedRows) {
    Add-Utf8Text -Path $OutputNotesPath -Text "$($FailedRow[2]): $($FailedRow[10])`r`n"
  }
}

Write-Host "Typed archive load timing summary written:"
Write-Host "  $OutputCsv"
Write-Host "  $OutputNotesPath"
