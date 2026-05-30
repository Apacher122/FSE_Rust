$BenchmarkUtf8NoBom = New-Object System.Text.UTF8Encoding($false)
$BenchmarkInvariantCulture = [System.Globalization.CultureInfo]::InvariantCulture

function Set-Utf8Text {
  param(
    [string]$Path,
    [string]$Text
  )

  [System.IO.File]::WriteAllText($Path, $Text, $BenchmarkUtf8NoBom)
}

function Add-Utf8Text {
  param(
    [string]$Path,
    [string]$Text
  )

  [System.IO.File]::AppendAllText($Path, $Text, $BenchmarkUtf8NoBom)
}

function Convert-ToInvariantDouble {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return $null
  }

  $TextValue = $Value.ToString().Trim()

  if ([string]::IsNullOrWhiteSpace($TextValue)) {
    return $null
  }

  $LowerTextValue = $TextValue.ToLowerInvariant()

  if ($LowerTextValue -eq "unavailable" -or $LowerTextValue -eq "not available" -or $LowerTextValue -eq "not_provided" -or $LowerTextValue -eq "not provided" -or $LowerTextValue -eq "n/a" -or $LowerTextValue -eq "na" -or $LowerTextValue -eq "none" -or $LowerTextValue -eq "null") {
    return $null
  }

  if ($LowerTextValue.EndsWith("x")) {
    $TextValue = $TextValue.Substring(0, $TextValue.Length - 1).Trim()

    if ([string]::IsNullOrWhiteSpace($TextValue)) {
      return $null
    }

    $LowerTextValue = $TextValue.ToLowerInvariant()
  }

  if ($LowerTextValue -eq "inf" -or $LowerTextValue -eq "+inf" -or $LowerTextValue -eq "infinity" -or $LowerTextValue -eq "+infinity") {
    return [double]::PositiveInfinity
  }

  if ($LowerTextValue -eq "-inf" -or $LowerTextValue -eq "-infinity") {
    return [double]::NegativeInfinity
  }

  if ($LowerTextValue -eq "nan") {
    return [double]::NaN
  }

  [double]$ParsedValue = 0.0

  if ([double]::TryParse($TextValue, [System.Globalization.NumberStyles]::Float, $BenchmarkInvariantCulture, [ref]$ParsedValue)) {
    return $ParsedValue
  }

  return $null
}

function Convert-ToInvariantDoubleOrZero {
  param(
    [object]$Value
  )

  $ConvertedValue = Convert-ToInvariantDouble -Value $Value

  if ($null -eq $ConvertedValue) {
    return 0.0
  }

  return $ConvertedValue
}

function Convert-ToInvariantIntOrZero {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return 0
  }

  if ([string]::IsNullOrWhiteSpace($Value.ToString())) {
    return 0
  }

  return [System.Convert]::ToInt64($Value, $BenchmarkInvariantCulture)
}

function Format-InvariantDouble {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return "unavailable"
  }

  try {
    $DoubleValue = Convert-ToInvariantDouble -Value $Value

    if ($null -eq $DoubleValue) {
      return "unavailable"
    }

    if ([double]::IsPositiveInfinity($DoubleValue)) {
      return "inf"
    }

    if ([double]::IsNegativeInfinity($DoubleValue)) {
      return "-inf"
    }

    if ([double]::IsNaN($DoubleValue)) {
      return "nan"
    }

    return $DoubleValue.ToString("0.000000", $BenchmarkInvariantCulture)
  }
  catch {
    return "unavailable"
  }
}

function Escape-CsvField {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return ""
  }

  $Text = $Value.ToString()

  if ($Text.Contains(",") -or $Text.Contains('"') -or $Text.Contains("`n") -or $Text.Contains("`r")) {
    $Escaped = $Text.Replace('"', '""')
    return "`"$Escaped`""
  }

  return $Text
}

function Join-CsvFields {
  param(
    [object[]]$Fields
  )

  return (($Fields | ForEach-Object { Escape-CsvField -Value $_ }) -join ",")
}

function Convert-ToCsvRow {
  param(
    [object[]]$Values
  )

  return Join-CsvFields -Fields $Values
}

function Write-CsvDocument {
  param(
    [string]$Path,
    [object[]]$Header,
    [object[]]$Rows
  )

  Set-Utf8Text -Path $Path -Text "$(Join-CsvFields -Fields $Header)`r`n"

  foreach ($Row in $Rows) {
    Add-Utf8Text -Path $Path -Text "$(Join-CsvFields -Fields $Row)`r`n"
  }
}


function Get-BaselineRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName
  )

  return $Rows | Where-Object { $_.baseline_name -eq $BaselineName } | Select-Object -First 1
}

function Get-Metric {
  param(
    [object]$Row,
    [string]$FieldName
  )

  if ($null -eq $Row) {
    return $null
  }

  $RawValue = $Row.$FieldName

  if ([string]::IsNullOrWhiteSpace($RawValue)) {
    return $null
  }

  return Convert-ToInvariantDouble -Value $RawValue
}

function Get-TextField {
  param(
    [object]$Row,
    [string]$FieldName
  )

  if ($null -eq $Row) {
    return "unavailable"
  }

  $RawValue = $Row.$FieldName

  if ($null -eq $RawValue) {
    return "unavailable"
  }

  if ([string]::IsNullOrWhiteSpace($RawValue.ToString())) {
    return "unavailable"
  }

  return $RawValue.ToString()
}

function Get-LowerTextField {
  param(
    [object]$Row,
    [string]$FieldName
  )

  $Text = Get-TextField -Row $Row -FieldName $FieldName

  if ($Text -eq "unavailable") {
    return $Text
  }

  return $Text.ToString().Trim().ToLowerInvariant()
}

function Get-Delta {
  param(
    [object]$CurrentValue,
    [object]$PreviousValue
  )

  if ($null -eq $CurrentValue -or $null -eq $PreviousValue) {
    return $null
  }

  return [double]$CurrentValue - [double]$PreviousValue
}

function Get-Classification {
  param(
    [object]$Delta,
    [double]$Threshold
  )

  if ($null -eq $Delta) {
    return "unavailable"
  }

  if ($Delta -ge $Threshold) {
    return "improved"
  }

  if ($Delta -le (-1.0 * $Threshold)) {
    return "regressed"
  }

  return "stable/noise"
}

function Get-Mean {
  param(
    [object[]]$Values
  )

  if ($Values.Count -eq 0) {
    return $null
  }

  $Sum = 0.0

  foreach ($Value in $Values) {
    $Sum += [double]$Value
  }

  return $Sum / $Values.Count
}

function Get-Minimum {
  param(
    [object[]]$Values
  )

  if ($Values.Count -eq 0) {
    return $null
  }

  $Minimum = [double]$Values[0]

  foreach ($Value in $Values) {
    $DoubleValue = [double]$Value

    if ($DoubleValue -lt $Minimum) {
      $Minimum = $DoubleValue
    }
  }

  return $Minimum
}

function Get-Maximum {
  param(
    [object[]]$Values
  )

  if ($Values.Count -eq 0) {
    return $null
  }

  $Maximum = [double]$Values[0]

  foreach ($Value in $Values) {
    $DoubleValue = [double]$Value

    if ($DoubleValue -gt $Maximum) {
      $Maximum = $DoubleValue
    }
  }

  return $Maximum
}

function Get-SampleStdDev {
  param(
    [object[]]$Values,
    [object]$Mean
  )

  if ($Values.Count -lt 2 -or $null -eq $Mean) {
    return 0.0
  }

  $SquaredDeltaSum = 0.0

  foreach ($Value in $Values) {
    $Delta = [double]$Value - [double]$Mean
    $SquaredDeltaSum += $Delta * $Delta
  }

  return [Math]::Sqrt($SquaredDeltaSum / ($Values.Count - 1))
}