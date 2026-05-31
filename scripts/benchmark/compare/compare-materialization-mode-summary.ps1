param(
  [string]$Label = "local",
  [string]$PreviousMaterializationSummaryCsv,
  [string]$CurrentMaterializationSummaryCsv,
  [string]$OutputDir = "benchmark_artifacts",
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$BenchmarkCsvLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-csv.ps1"
$BenchmarkComparisonLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-comparison.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkComparisonLibrary)) {
  throw "benchmark comparison helper library was not found: $BenchmarkComparisonLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkComparisonLibrary

$ComparisonInputs = Import-BenchmarkComparisonCsvPair `
  -PreviousPath $PreviousMaterializationSummaryCsv `
  -CurrentPath $CurrentMaterializationSummaryCsv `
  -PreviousParameterName "PreviousMaterializationSummaryCsv" `
  -CurrentParameterName "CurrentMaterializationSummaryCsv" `
  -PreviousDescription "previous materialization summary" `
  -CurrentDescription "current materialization summary"

$ComparisonPaths = New-BenchmarkComparisonOutputPathSet `
  -OutputDir $OutputDir `
  -Label $Label `
  -CsvFileName "materialization-mode-summary-comparison-$Label.csv" `
  -NotesFileName "materialization-mode-summary-comparison-$Label.txt"

$ComparisonCsvPath = $ComparisonPaths.CsvPath
$ComparisonNotesPath = $ComparisonPaths.NotesPath


function Require-MaterializationCsvField {
  param(
    [object]$Row,
    [string]$FieldName,
    [string]$CsvDescription
  )

  if ($null -eq $Row) {
    throw "$CsvDescription CSV has no rows"
  }

  if ($null -eq $Row.PSObject.Properties[$FieldName]) {
    throw "$CsvDescription CSV is missing required field: $FieldName"
  }
}

function Get-OptionalMaterializationCsvField {
  param(
    [object]$Row,
    [string]$FieldName
  )

  if ($null -eq $Row) {
    return $null
  }

  if ($null -eq $Row.PSObject.Properties[$FieldName]) {
    return $null
  }

  return $Row.$FieldName
}

function Convert-MaterializationDurationToNanoseconds {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return $null
  }

  $Text = $Value.ToString().Trim().ToLowerInvariant()

  if ([string]::IsNullOrWhiteSpace($Text) -or $Text -eq "unavailable") {
    return $null
  }

  $Match = [regex]::Match($Text, "^([0-9]+(?:\.[0-9]+)?)(ns|us|µs|ms|s)$")

  if (!$Match.Success) {
    throw "unsupported duration value in materialization summary: $Value"
  }

  $Magnitude = Convert-ToInvariantDouble -Value $Match.Groups[1].Value

  if ($null -eq $Magnitude) {
    return $null
  }

  $Unit = $Match.Groups[2].Value

  switch ($Unit) {
    "ns" { return $Magnitude }
    "us" { return $Magnitude * 1000.0 }
    "µs" { return $Magnitude * 1000.0 }
    "ms" { return $Magnitude * 1000000.0 }
    "s" { return $Magnitude * 1000000000.0 }
    default { throw "unsupported duration unit in materialization summary: $Unit" }
  }
}

function Convert-MaterializationSpeedupToRatio {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return $null
  }

  $Text = $Value.ToString().Trim().ToLowerInvariant()

  if ([string]::IsNullOrWhiteSpace($Text) -or $Text -eq "unavailable") {
    return $null
  }

  return Convert-ToInvariantDouble -Value $Text
}

function Get-RelativeDeltaClassification {
  param(
    [object]$Delta,
    [object]$ReferenceValue,
    [double]$NoiseThreshold,
    [bool]$HigherIsBetter
  )

  if ($null -eq $Delta -or $null -eq $ReferenceValue) {
    return "unavailable"
  }

  $ReferenceMagnitude = [Math]::Abs([double]$ReferenceValue)

  if ($ReferenceMagnitude -eq 0.0) {
    if ([double]$Delta -eq 0.0) {
      return "stable/noise"
    }

    if ($HigherIsBetter) {
      if ([double]$Delta -gt 0.0) {
        return "improved"
      }

      return "regressed"
    }

    if ([double]$Delta -lt 0.0) {
      return "improved"
    }

    return "regressed"
  }

  $RelativeDelta = [double]$Delta / $ReferenceMagnitude

  if ([Math]::Abs($RelativeDelta) -le $NoiseThreshold) {
    return "stable/noise"
  }

  if ($HigherIsBetter) {
    if ($RelativeDelta -gt 0.0) {
      return "improved"
    }

    return "regressed"
  }

  if ($RelativeDelta -lt 0.0) {
    return "improved"
  }

  return "regressed"
}

function New-MaterializationSummaryRowMap {
  param(
    [object[]]$Rows
  )

  $Map = @{}

  foreach ($Row in $Rows) {
    $Dataset = Get-LowerTextField -Row $Row -FieldName "dataset"
    $MaxDepth = Get-TextField -Row $Row -FieldName "max_depth"
    $WorkloadName = Get-TextField -Row $Row -FieldName "workload_name"
    $Key = "$Dataset|$MaxDepth|$WorkloadName"

    if ($Map.ContainsKey($Key)) {
      throw "duplicate materialization summary key found: $Key"
    }

    $Map[$Key] = $Row
  }

  return $Map
}

function New-ComparisonRow {
  param(
    [object]$PreviousRow,
    [object]$CurrentRow
  )

  $PreviousMatched = Convert-ToInvariantIntOrZero -Value $PreviousRow.matched_records
  $CurrentMatched = Convert-ToInvariantIntOrZero -Value $CurrentRow.matched_records
  $MatchedDelta = $CurrentMatched - $PreviousMatched

  $PreviousFreshOwnedNs = Convert-MaterializationDurationToNanoseconds -Value $PreviousRow.fresh_owned_elapsed
  $CurrentFreshOwnedNs = Convert-MaterializationDurationToNanoseconds -Value $CurrentRow.fresh_owned_elapsed
  $FreshOwnedDeltaNs = Get-Delta -CurrentValue $CurrentFreshOwnedNs -PreviousValue $PreviousFreshOwnedNs

  $PreviousReusableOwnedNs = Convert-MaterializationDurationToNanoseconds -Value $PreviousRow.reusable_owned_elapsed
  $CurrentReusableOwnedNs = Convert-MaterializationDurationToNanoseconds -Value $CurrentRow.reusable_owned_elapsed
  $ReusableOwnedDeltaNs = Get-Delta -CurrentValue $CurrentReusableOwnedNs -PreviousValue $PreviousReusableOwnedNs

  $PreviousReferenceNs = Convert-MaterializationDurationToNanoseconds -Value $PreviousRow.reference_elapsed
  $CurrentReferenceNs = Convert-MaterializationDurationToNanoseconds -Value $CurrentRow.reference_elapsed
  $ReferenceDeltaNs = Get-Delta -CurrentValue $CurrentReferenceNs -PreviousValue $PreviousReferenceNs

  $PreviousVisitorNs = Convert-MaterializationDurationToNanoseconds `
    -Value (Get-OptionalMaterializationCsvField -Row $PreviousRow -FieldName "visitor_elapsed")
  $CurrentVisitorNs = Convert-MaterializationDurationToNanoseconds `
    -Value (Get-OptionalMaterializationCsvField -Row $CurrentRow -FieldName "visitor_elapsed")
  $VisitorDeltaNs = Get-Delta -CurrentValue $CurrentVisitorNs -PreviousValue $PreviousVisitorNs

  $PreviousCountOnlyNs = Convert-MaterializationDurationToNanoseconds -Value $PreviousRow.count_only_elapsed
  $CurrentCountOnlyNs = Convert-MaterializationDurationToNanoseconds -Value $CurrentRow.count_only_elapsed
  $CountOnlyDeltaNs = Get-Delta -CurrentValue $CurrentCountOnlyNs -PreviousValue $PreviousCountOnlyNs

  $PreviousCountSpeedup = Convert-MaterializationSpeedupToRatio -Value $PreviousRow.count_speedup
  $CurrentCountSpeedup = Convert-MaterializationSpeedupToRatio -Value $CurrentRow.count_speedup
  $CountSpeedupDelta = Get-Delta -CurrentValue $CurrentCountSpeedup -PreviousValue $PreviousCountSpeedup

  $PreviousReferenceSpeedup = Convert-MaterializationSpeedupToRatio -Value $PreviousRow.reference_speedup
  $CurrentReferenceSpeedup = Convert-MaterializationSpeedupToRatio -Value $CurrentRow.reference_speedup
  $ReferenceSpeedupDelta = Get-Delta -CurrentValue $CurrentReferenceSpeedup -PreviousValue $PreviousReferenceSpeedup

  $PreviousVisitorSpeedup = Convert-MaterializationSpeedupToRatio `
    -Value (Get-OptionalMaterializationCsvField -Row $PreviousRow -FieldName "visitor_speedup")
  $CurrentVisitorSpeedup = Convert-MaterializationSpeedupToRatio `
    -Value (Get-OptionalMaterializationCsvField -Row $CurrentRow -FieldName "visitor_speedup")
  $VisitorSpeedupDelta = Get-Delta -CurrentValue $CurrentVisitorSpeedup -PreviousValue $PreviousVisitorSpeedup

  $PreviousReusableSpeedup = Convert-MaterializationSpeedupToRatio -Value $PreviousRow.reusable_speedup
  $CurrentReusableSpeedup = Convert-MaterializationSpeedupToRatio -Value $CurrentRow.reusable_speedup
  $ReusableSpeedupDelta = Get-Delta -CurrentValue $CurrentReusableSpeedup -PreviousValue $PreviousReusableSpeedup

  $PreviousAgreement = Get-LowerTextField -Row $PreviousRow -FieldName "agreement"
  $CurrentAgreement = Get-LowerTextField -Row $CurrentRow -FieldName "agreement"

  $AgreementStatus = if ($PreviousAgreement -eq "pass" -and $CurrentAgreement -eq "pass" -and $MatchedDelta -eq 0) {
    "pass"
  }
  else {
    "review"
  }

  return [PSCustomObject]@{
    Dataset                            = Get-LowerTextField -Row $CurrentRow -FieldName "dataset"
    MaxDepth                           = Get-TextField -Row $CurrentRow -FieldName "max_depth"
    WorkloadName                       = Get-TextField -Row $CurrentRow -FieldName "workload_name"

    PreviousMatchedRecords             = $PreviousMatched
    CurrentMatchedRecords              = $CurrentMatched
    MatchedRecordsDelta                = $MatchedDelta

    PreviousFreshOwnedElapsedNs        = $PreviousFreshOwnedNs
    CurrentFreshOwnedElapsedNs         = $CurrentFreshOwnedNs
    FreshOwnedElapsedDeltaNs           = $FreshOwnedDeltaNs
    FreshOwnedElapsedClassification    = Get-RelativeDeltaClassification `
      -Delta $FreshOwnedDeltaNs `
      -ReferenceValue $PreviousFreshOwnedNs `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $false

    PreviousReusableOwnedElapsedNs     = $PreviousReusableOwnedNs
    CurrentReusableOwnedElapsedNs      = $CurrentReusableOwnedNs
    ReusableOwnedElapsedDeltaNs        = $ReusableOwnedDeltaNs
    ReusableOwnedElapsedClassification = Get-RelativeDeltaClassification `
      -Delta $ReusableOwnedDeltaNs `
      -ReferenceValue $PreviousReusableOwnedNs `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $false

    PreviousReferenceElapsedNs         = $PreviousReferenceNs
    CurrentReferenceElapsedNs          = $CurrentReferenceNs
    ReferenceElapsedDeltaNs            = $ReferenceDeltaNs
    ReferenceElapsedClassification     = Get-RelativeDeltaClassification `
      -Delta $ReferenceDeltaNs `
      -ReferenceValue $PreviousReferenceNs `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $false

    PreviousVisitorElapsedNs           = $PreviousVisitorNs
    CurrentVisitorElapsedNs            = $CurrentVisitorNs
    VisitorElapsedDeltaNs              = $VisitorDeltaNs
    VisitorElapsedClassification       = Get-RelativeDeltaClassification `
      -Delta $VisitorDeltaNs `
      -ReferenceValue $PreviousVisitorNs `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $false

    PreviousCountOnlyElapsedNs         = $PreviousCountOnlyNs
    CurrentCountOnlyElapsedNs          = $CurrentCountOnlyNs
    CountOnlyElapsedDeltaNs            = $CountOnlyDeltaNs
    CountOnlyElapsedClassification     = Get-RelativeDeltaClassification `
      -Delta $CountOnlyDeltaNs `
      -ReferenceValue $PreviousCountOnlyNs `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $false

    PreviousCountSpeedup               = $PreviousCountSpeedup
    CurrentCountSpeedup                = $CurrentCountSpeedup
    CountSpeedupDelta                  = $CountSpeedupDelta
    CountSpeedupClassification         = Get-RelativeDeltaClassification `
      -Delta $CountSpeedupDelta `
      -ReferenceValue $PreviousCountSpeedup `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $true

    PreviousReferenceSpeedup           = $PreviousReferenceSpeedup
    CurrentReferenceSpeedup            = $CurrentReferenceSpeedup
    ReferenceSpeedupDelta              = $ReferenceSpeedupDelta
    ReferenceSpeedupClassification     = Get-RelativeDeltaClassification `
      -Delta $ReferenceSpeedupDelta `
      -ReferenceValue $PreviousReferenceSpeedup `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $true

    PreviousVisitorSpeedup             = $PreviousVisitorSpeedup
    CurrentVisitorSpeedup              = $CurrentVisitorSpeedup
    VisitorSpeedupDelta                = $VisitorSpeedupDelta
    VisitorSpeedupClassification       = Get-RelativeDeltaClassification `
      -Delta $VisitorSpeedupDelta `
      -ReferenceValue $PreviousVisitorSpeedup `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $true

    PreviousReusableSpeedup            = $PreviousReusableSpeedup
    CurrentReusableSpeedup             = $CurrentReusableSpeedup
    ReusableSpeedupDelta               = $ReusableSpeedupDelta
    ReusableSpeedupClassification      = Get-RelativeDeltaClassification `
      -Delta $ReusableSpeedupDelta `
      -ReferenceValue $PreviousReusableSpeedup `
      -NoiseThreshold $NoiseThreshold `
      -HigherIsBetter $true

    PreviousAgreement                  = $PreviousAgreement
    CurrentAgreement                   = $CurrentAgreement
    AgreementStatus                    = $AgreementStatus
  }
}

$PreviousRows = @($ComparisonInputs.PreviousRows)
$CurrentRows = @($ComparisonInputs.CurrentRows)

if ($PreviousRows.Count -eq 0) {
  throw "previous materialization summary CSV has no rows: $PreviousMaterializationSummaryCsv"
}

if ($CurrentRows.Count -eq 0) {
  throw "current materialization summary CSV has no rows: $CurrentMaterializationSummaryCsv"
}

$RequiredFields = @(
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "fresh_owned_elapsed",
  "reusable_owned_elapsed",
  "reference_elapsed",
  "count_only_elapsed",
  "count_speedup",
  "reference_speedup",
  "reusable_speedup",
  "agreement"
)

foreach ($FieldName in $RequiredFields) {
  Require-MaterializationCsvField `
    -Row $PreviousRows[0] `
    -FieldName $FieldName `
    -CsvDescription "previous materialization summary"

  Require-MaterializationCsvField `
    -Row $CurrentRows[0] `
    -FieldName $FieldName `
    -CsvDescription "current materialization summary"
}

$PreviousMap = New-MaterializationSummaryRowMap -Rows $PreviousRows
$CurrentMap = New-MaterializationSummaryRowMap -Rows $CurrentRows

$ComparisonRows = New-Object System.Collections.Generic.List[object]
$MissingPreviousKeys = New-Object System.Collections.Generic.List[string]
$MissingCurrentKeys = New-Object System.Collections.Generic.List[string]

foreach ($Key in $CurrentMap.Keys) {
  if (!$PreviousMap.ContainsKey($Key)) {
    $MissingPreviousKeys.Add($Key)
    continue
  }

  $ComparisonRows.Add((New-ComparisonRow `
        -PreviousRow $PreviousMap[$Key] `
        -CurrentRow $CurrentMap[$Key]))
}

foreach ($Key in $PreviousMap.Keys) {
  if (!$CurrentMap.ContainsKey($Key)) {
    $MissingCurrentKeys.Add($Key)
  }
}

if ($ComparisonRows.Count -eq 0) {
  throw "materialization comparison produced no comparable rows"
}

$Header = @(
  "dataset",
  "max_depth",
  "workload_name",
  "previous_matched_records",
  "current_matched_records",
  "matched_records_delta",
  "previous_fresh_owned_elapsed_ns",
  "current_fresh_owned_elapsed_ns",
  "fresh_owned_elapsed_delta_ns",
  "fresh_owned_elapsed_classification",
  "previous_reusable_owned_elapsed_ns",
  "current_reusable_owned_elapsed_ns",
  "reusable_owned_elapsed_delta_ns",
  "reusable_owned_elapsed_classification",
  "previous_reference_elapsed_ns",
  "current_reference_elapsed_ns",
  "reference_elapsed_delta_ns",
  "reference_elapsed_classification",
  "previous_visitor_elapsed_ns",
  "current_visitor_elapsed_ns",
  "visitor_elapsed_delta_ns",
  "visitor_elapsed_classification",
  "previous_count_only_elapsed_ns",
  "current_count_only_elapsed_ns",
  "count_only_elapsed_delta_ns",
  "count_only_elapsed_classification",
  "previous_count_speedup",
  "current_count_speedup",
  "count_speedup_delta",
  "count_speedup_classification",
  "previous_reference_speedup",
  "current_reference_speedup",
  "reference_speedup_delta",
  "reference_speedup_classification",
  "previous_visitor_speedup",
  "current_visitor_speedup",
  "visitor_speedup_delta",
  "visitor_speedup_classification",
  "previous_reusable_speedup",
  "current_reusable_speedup",
  "reusable_speedup_delta",
  "reusable_speedup_classification",
  "previous_agreement",
  "current_agreement",
  "agreement_status"
)

$CsvRows = New-Object System.Collections.Generic.List[object]

foreach ($Row in $ComparisonRows) {
  $CsvRows.Add(@(
      $Row.Dataset,
      $Row.MaxDepth,
      $Row.WorkloadName,
      $Row.PreviousMatchedRecords,
      $Row.CurrentMatchedRecords,
      $Row.MatchedRecordsDelta,
      (Format-InvariantDouble -Value $Row.PreviousFreshOwnedElapsedNs),
      (Format-InvariantDouble -Value $Row.CurrentFreshOwnedElapsedNs),
      (Format-InvariantDouble -Value $Row.FreshOwnedElapsedDeltaNs),
      $Row.FreshOwnedElapsedClassification,
      (Format-InvariantDouble -Value $Row.PreviousReusableOwnedElapsedNs),
      (Format-InvariantDouble -Value $Row.CurrentReusableOwnedElapsedNs),
      (Format-InvariantDouble -Value $Row.ReusableOwnedElapsedDeltaNs),
      $Row.ReusableOwnedElapsedClassification,
      (Format-InvariantDouble -Value $Row.PreviousReferenceElapsedNs),
      (Format-InvariantDouble -Value $Row.CurrentReferenceElapsedNs),
      (Format-InvariantDouble -Value $Row.ReferenceElapsedDeltaNs),
      $Row.ReferenceElapsedClassification,
      (Format-InvariantDouble -Value $Row.PreviousVisitorElapsedNs),
      (Format-InvariantDouble -Value $Row.CurrentVisitorElapsedNs),
      (Format-InvariantDouble -Value $Row.VisitorElapsedDeltaNs),
      $Row.VisitorElapsedClassification,
      (Format-InvariantDouble -Value $Row.PreviousCountOnlyElapsedNs),
      (Format-InvariantDouble -Value $Row.CurrentCountOnlyElapsedNs),
      (Format-InvariantDouble -Value $Row.CountOnlyElapsedDeltaNs),
      $Row.CountOnlyElapsedClassification,
      (Format-InvariantDouble -Value $Row.PreviousCountSpeedup),
      (Format-InvariantDouble -Value $Row.CurrentCountSpeedup),
      (Format-InvariantDouble -Value $Row.CountSpeedupDelta),
      $Row.CountSpeedupClassification,
      (Format-InvariantDouble -Value $Row.PreviousReferenceSpeedup),
      (Format-InvariantDouble -Value $Row.CurrentReferenceSpeedup),
      (Format-InvariantDouble -Value $Row.ReferenceSpeedupDelta),
      $Row.ReferenceSpeedupClassification,
      (Format-InvariantDouble -Value $Row.PreviousVisitorSpeedup),
      (Format-InvariantDouble -Value $Row.CurrentVisitorSpeedup),
      (Format-InvariantDouble -Value $Row.VisitorSpeedupDelta),
      $Row.VisitorSpeedupClassification,
      (Format-InvariantDouble -Value $Row.PreviousReusableSpeedup),
      (Format-InvariantDouble -Value $Row.CurrentReusableSpeedup),
      (Format-InvariantDouble -Value $Row.ReusableSpeedupDelta),
      $Row.ReusableSpeedupClassification,
      $Row.PreviousAgreement,
      $Row.CurrentAgreement,
      $Row.AgreementStatus
    ))
}

Write-CsvDocument -Path $ComparisonCsvPath -Header $Header -Rows $CsvRows

$AgreementReviewRows = @($ComparisonRows | Where-Object { $_.AgreementStatus -ne "pass" })
$MatchedDeltaRows = @($ComparisonRows | Where-Object { $_.MatchedRecordsDelta -ne 0 })

$FreshImproved = @($ComparisonRows | Where-Object { $_.FreshOwnedElapsedClassification -eq "improved" })
$FreshStable = @($ComparisonRows | Where-Object { $_.FreshOwnedElapsedClassification -eq "stable/noise" })
$FreshRegressed = @($ComparisonRows | Where-Object { $_.FreshOwnedElapsedClassification -eq "regressed" })

$ReusableImproved = @($ComparisonRows | Where-Object { $_.ReusableOwnedElapsedClassification -eq "improved" })
$ReusableStable = @($ComparisonRows | Where-Object { $_.ReusableOwnedElapsedClassification -eq "stable/noise" })
$ReusableRegressed = @($ComparisonRows | Where-Object { $_.ReusableOwnedElapsedClassification -eq "regressed" })

$ReferenceImproved = @($ComparisonRows | Where-Object { $_.ReferenceElapsedClassification -eq "improved" })
$ReferenceStable = @($ComparisonRows | Where-Object { $_.ReferenceElapsedClassification -eq "stable/noise" })
$ReferenceRegressed = @($ComparisonRows | Where-Object { $_.ReferenceElapsedClassification -eq "regressed" })

$VisitorImproved = @($ComparisonRows | Where-Object { $_.VisitorElapsedClassification -eq "improved" })
$VisitorStable = @($ComparisonRows | Where-Object { $_.VisitorElapsedClassification -eq "stable/noise" })
$VisitorRegressed = @($ComparisonRows | Where-Object { $_.VisitorElapsedClassification -eq "regressed" })

$CountOnlyImproved = @($ComparisonRows | Where-Object { $_.CountOnlyElapsedClassification -eq "improved" })
$CountOnlyStable = @($ComparisonRows | Where-Object { $_.CountOnlyElapsedClassification -eq "stable/noise" })
$CountOnlyRegressed = @($ComparisonRows | Where-Object { $_.CountOnlyElapsedClassification -eq "regressed" })

Set-Utf8Text -Path $ComparisonNotesPath -Text "Materialization mode summary comparison`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "=======================================`r`n`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Previous summary CSV: $PreviousMaterializationSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Current summary CSV: $CurrentMaterializationSummaryCsv`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Output CSV: $ComparisonCsvPath`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Elapsed classifications use relative delta; lower elapsed is better.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Speedup classifications use relative delta; higher speedup is better.`r`n"

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nRow coverage`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "comparison rows: $($ComparisonRows.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "missing previous rows: $($MissingPreviousKeys.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "missing current rows: $($MissingCurrentKeys.Count)`r`n"

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nExactness agreement`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "-------------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "agreement review rows: $($AgreementReviewRows.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "matched-record delta rows: $($MatchedDeltaRows.Count)`r`n"

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nElapsed movement`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "----------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "fresh owned improved/stable/regressed: $($FreshImproved.Count)/$($FreshStable.Count)/$($FreshRegressed.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "reusable owned improved/stable/regressed: $($ReusableImproved.Count)/$($ReusableStable.Count)/$($ReusableRegressed.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "reference-result improved/stable/regressed: $($ReferenceImproved.Count)/$($ReferenceStable.Count)/$($ReferenceRegressed.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "reference-visitor improved/stable/regressed: $($VisitorImproved.Count)/$($VisitorStable.Count)/$($VisitorRegressed.Count)`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "count-only improved/stable/regressed: $($CountOnlyImproved.Count)/$($CountOnlyStable.Count)/$($CountOnlyRegressed.Count)`r`n"

if ($MissingPreviousKeys.Count -gt 0) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nMissing previous rows`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "---------------------`r`n"

  foreach ($Key in $MissingPreviousKeys) {
    Add-Utf8Text -Path $ComparisonNotesPath -Text "$Key`r`n"
  }
}

if ($MissingCurrentKeys.Count -gt 0) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nMissing current rows`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "--------------------`r`n"

  foreach ($Key in $MissingCurrentKeys) {
    Add-Utf8Text -Path $ComparisonNotesPath -Text "$Key`r`n"
  }
}

if ($AgreementReviewRows.Count -gt 0) {
  Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nRows needing exactness review`r`n"
  Add-Utf8Text -Path $ComparisonNotesPath -Text "-----------------------------`r`n"

  foreach ($Row in $AgreementReviewRows) {
    Add-Utf8Text -Path $ComparisonNotesPath -Text "$($Row.Dataset)|$($Row.MaxDepth)|$($Row.WorkloadName): $($Row.AgreementStatus)`r`n"
  }
}

Add-Utf8Text -Path $ComparisonNotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Keep exactness agreement separate from timing movement.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Treat elapsed movement outside the threshold as a review signal, not an automatic keep/revert decision.`r`n"
Add-Utf8Text -Path $ComparisonNotesPath -Text "Use repeated benchmark evidence before making performance claims.`r`n"

Write-Host "Materialization mode summary comparison written:"
Write-Host "  $ComparisonCsvPath"
Write-Host "  $ComparisonNotesPath"
