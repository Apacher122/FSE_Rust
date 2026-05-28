$BenchmarkCsvLibrary = Join-Path $PSScriptRoot "benchmark-csv.ps1"

if (!(Get-Command Convert-ToInvariantDouble -ErrorAction SilentlyContinue)) {
  if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
    throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
  }

  . $BenchmarkCsvLibrary
}

function Get-LowGapRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName
  )

  return Get-BaselineRow -Rows $Rows -BaselineName $BaselineName
}

function Get-LowGapMetric {
  param(
    [object]$Row,
    [string]$FieldName
  )

  return Get-Metric -Row $Row -FieldName $FieldName
}

function Get-WorkloadMetric {
  param(
    [object]$Row,
    [string]$FieldName
  )

  return Get-Metric -Row $Row -FieldName $FieldName
}

function Get-RatioOrNull {
  param(
    [object]$Numerator,
    [object]$Denominator
  )

  if ($null -eq $Numerator -or $null -eq $Denominator) {
    return $null
  }

  $NumeratorValue = [double]$Numerator
  $DenominatorValue = [double]$Denominator

  if ($DenominatorValue -eq 0.0) {
    return $null
  }

  return $NumeratorValue / $DenominatorValue
}

function Get-LowGapClassification {
  param(
    [object]$Delta,
    [double]$Threshold
  )

  return Get-Classification -Delta $Delta -Threshold $Threshold
}
