$BenchmarkCsvLibrary = Join-Path $PSScriptRoot "benchmark-csv.ps1"

if (!(Get-Command Get-BaselineRow -ErrorAction SilentlyContinue)) {
  if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
    throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
  }

  . $BenchmarkCsvLibrary
}

function Get-BenchmarkTreeBaselineNames {
  return @("kd_tree", "r_tree")
}


function Resolve-BenchmarkComparisonCsvPath {
  param(
    [string]$Path,
    [string]$ParameterName,
    [string]$Description
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    throw "`-$ParameterName` is required"
  }

  if (!(Test-Path -LiteralPath $Path)) {
    throw "$Description CSV was not found: $Path"
  }

  return (Resolve-Path -LiteralPath $Path).Path
}

function Import-BenchmarkComparisonCsvRows {
  param(
    [string]$Path,
    [string]$ParameterName,
    [string]$Description
  )

  $ResolvedPath = Resolve-BenchmarkComparisonCsvPath `
    -Path $Path `
    -ParameterName $ParameterName `
    -Description $Description

  return [PSCustomObject]@{
    InputPath    = $Path
    ResolvedPath = $ResolvedPath
    Rows         = @(Import-Csv -LiteralPath $ResolvedPath)
  }
}

function Import-BenchmarkComparisonCsvPair {
  param(
    [string]$PreviousPath,
    [string]$CurrentPath,
    [string]$PreviousParameterName,
    [string]$CurrentParameterName,
    [string]$PreviousDescription,
    [string]$CurrentDescription
  )

  $PreviousInput = Import-BenchmarkComparisonCsvRows `
    -Path $PreviousPath `
    -ParameterName $PreviousParameterName `
    -Description $PreviousDescription

  $CurrentInput = Import-BenchmarkComparisonCsvRows `
    -Path $CurrentPath `
    -ParameterName $CurrentParameterName `
    -Description $CurrentDescription

  return [PSCustomObject]@{
    PreviousInput        = $PreviousInput
    CurrentInput         = $CurrentInput
    PreviousRows         = $PreviousInput.Rows
    CurrentRows          = $CurrentInput.Rows
    PreviousResolvedPath = $PreviousInput.ResolvedPath
    CurrentResolvedPath  = $CurrentInput.ResolvedPath
  }
}

function New-BenchmarkComparisonOutputPathSet {
  param(
    [string]$OutputDir,
    [string]$Label,
    [string]$CsvFileName,
    [string]$NotesFileName
  )

  New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

  return [PSCustomObject]@{
    CsvPath   = Join-Path $OutputDir $CsvFileName
    NotesPath = Join-Path $OutputDir $NotesFileName
  }
}

function Get-BenchmarkSummaryRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName
  )

  return Get-BaselineRow -Rows $Rows -BaselineName $BaselineName
}

function New-BenchmarkBaselineComparisonObjects {
  param(
    [object[]]$PreviousRows,
    [object[]]$CurrentRows,
    [string[]]$BaselineNames,
    [scriptblock]$RowFactory
  )

  $ComparisonObjects = @()

  foreach ($BaselineName in $BaselineNames) {
    $PreviousRow = Get-BenchmarkSummaryRow -Rows $PreviousRows -BaselineName $BaselineName
    $CurrentRow = Get-BenchmarkSummaryRow -Rows $CurrentRows -BaselineName $BaselineName

    $ComparisonObjects += & $RowFactory $BaselineName $PreviousRow $CurrentRow
  }

  return $ComparisonObjects
}
