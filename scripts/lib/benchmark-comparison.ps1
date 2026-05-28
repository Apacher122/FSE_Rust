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
