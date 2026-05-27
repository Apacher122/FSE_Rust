function Get-BenchmarkTreeBaselineNames {
  return @("kd_tree", "r_tree")
}

function Get-BenchmarkSummaryRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName
  )

  return $Rows | Where-Object { $_.baseline_name -eq $BaselineName } | Select-Object -First 1
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
