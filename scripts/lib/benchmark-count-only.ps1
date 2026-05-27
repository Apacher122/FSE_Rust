function Require-CountOnlyCsvField {
  param(
    [object]$Row,
    [string]$FieldName,
    [string]$CsvDescription
  )

  if ($null -eq $Row.PSObject.Properties[$FieldName]) {
    throw "required $CsvDescription CSV field was not found: $FieldName"
  }
}

function Get-CountOnlySelectivityBucket {
  param(
    [double]$CandidateRatio
  )

  if ($CandidateRatio -le 0.0) {
    return "empty"
  }

  if ($CandidateRatio -le 0.25) {
    return "low"
  }

  if ($CandidateRatio -le 0.50) {
    return "medium"
  }

  if ($CandidateRatio -lt 1.0) {
    return "high"
  }

  return "full"
}

function New-CountOnlySummaryKey {
  param(
    [string]$BaselineName,
    [string]$SelectivityBucket
  )

  return "$BaselineName|$SelectivityBucket"
}

function Get-CountOnlySummaryRowKey {
  param(
    [object]$Row
  )

  return New-CountOnlySummaryKey `
    -BaselineName $Row.baseline_name `
    -SelectivityBucket $Row.selectivity_bucket
}

function New-CountOnlySummaryRowMap {
  param(
    [object[]]$Rows
  )

  $Map = @{}

  foreach ($Row in $Rows) {
    $Map[(Get-CountOnlySummaryRowKey -Row $Row)] = $Row
  }

  return $Map
}

function Get-CountOnlyHigherIsBetterClassification {
  param(
    [object]$Delta,
    [double]$NoiseThreshold
  )

  if ($null -eq $Delta) {
    return "unavailable"
  }

  if ($Delta -ge $NoiseThreshold) {
    return "improved"
  }

  if ($Delta -le (-1.0 * $NoiseThreshold)) {
    return "regressed"
  }

  return "stable/noise"
}

function Get-CountOnlyLowerIsBetterClassification {
  param(
    [object]$Delta,
    [double]$ReferenceValue,
    [double]$NoiseThreshold
  )

  if ($null -eq $Delta) {
    return "unavailable"
  }

  if ($ReferenceValue -le 0.0) {
    return "unavailable"
  }

  $RelativeDelta = [double]$Delta / $ReferenceValue

  if ($RelativeDelta -le (-1.0 * $NoiseThreshold)) {
    return "improved"
  }

  if ($RelativeDelta -ge $NoiseThreshold) {
    return "regressed"
  }

  return "stable/noise"
}

function New-CountOnlySummaryRow {
  param(
    [object]$SourceRow,
    [string]$SelectivityBucket
  )

  return [PSCustomObject]@{
    DatasetRecords                     = $SourceRow.dataset_records
    TimingIterations                   = $SourceRow.timing_iterations
    MaxDepth                           = $SourceRow.max_depth
    IndexValid                         = $SourceRow.index_valid
    LeafCardinalityValid               = $SourceRow.leaf_cardinality_valid
    BaselineName                       = $SourceRow.baseline_name
    SelectivityBucket                  = $SelectivityBucket
    WorkloadCount                      = 0
    StatsAgreeCount                    = 0
    TotalFseReconstructedRecords       = 0
    TotalCountOnlyReconstructedRecords = 0
    TotalFseMatchedRecords             = 0
    TotalCountOnlyMatchedRecords       = 0
    CandidateRatioSum                  = 0.0
    OwnedAverageElapsedNsSum           = 0.0
    CountOnlyAverageElapsedNsSum       = 0.0
    OwnedResultOverheadNsSum           = 0.0
    CountOnlySpeedupRatioSum           = 0.0
  }
}

function New-CountOnlyCalculatedSummary {
  param(
    [object]$Summary
  )

  $WorkloadCount = [double]$Summary.WorkloadCount

  $MeanCandidateRatio = $Summary.CandidateRatioSum / $WorkloadCount
  $MeanOwnedAverageElapsedNs = $Summary.OwnedAverageElapsedNsSum / $WorkloadCount
  $MeanCountOnlyAverageElapsedNs = $Summary.CountOnlyAverageElapsedNsSum / $WorkloadCount
  $MeanOwnedResultOverheadNs = $Summary.OwnedResultOverheadNsSum / $WorkloadCount
  $MeanCountOnlySpeedupRatio = $Summary.CountOnlySpeedupRatioSum / $WorkloadCount

  if ($Summary.CountOnlyAverageElapsedNsSum -le 0.0) {
    $WeightedCountOnlySpeedupRatio = 0.0
  }
  else {
    $WeightedCountOnlySpeedupRatio =
    $Summary.OwnedAverageElapsedNsSum / $Summary.CountOnlyAverageElapsedNsSum
  }

  return [PSCustomObject]@{
    DatasetRecords                     = $Summary.DatasetRecords
    TimingIterations                   = $Summary.TimingIterations
    MaxDepth                           = $Summary.MaxDepth
    IndexValid                         = $Summary.IndexValid
    LeafCardinalityValid               = $Summary.LeafCardinalityValid
    BaselineName                       = $Summary.BaselineName
    SelectivityBucket                  = $Summary.SelectivityBucket
    WorkloadCount                      = $Summary.WorkloadCount
    StatsAgreeCount                    = $Summary.StatsAgreeCount
    AllStatsMatchOwned                 = $Summary.StatsAgreeCount -eq $Summary.WorkloadCount
    TotalFseReconstructedRecords       = $Summary.TotalFseReconstructedRecords
    TotalCountOnlyReconstructedRecords = $Summary.TotalCountOnlyReconstructedRecords
    TotalFseMatchedRecords             = $Summary.TotalFseMatchedRecords
    TotalCountOnlyMatchedRecords       = $Summary.TotalCountOnlyMatchedRecords
    MeanCandidateRatio                 = $MeanCandidateRatio
    MeanOwnedAverageElapsedNs          = $MeanOwnedAverageElapsedNs
    MeanCountOnlyAverageElapsedNs      = $MeanCountOnlyAverageElapsedNs
    MeanOwnedResultOverheadNs          = $MeanOwnedResultOverheadNs
    MeanCountOnlySpeedupRatio          = $MeanCountOnlySpeedupRatio
    WeightedCountOnlySpeedupRatio      = $WeightedCountOnlySpeedupRatio
    TotalOwnedAverageElapsedNs         = $Summary.OwnedAverageElapsedNsSum
    TotalCountOnlyAverageElapsedNs     = $Summary.CountOnlyAverageElapsedNsSum
    TotalOwnedResultOverheadNs         = $Summary.OwnedResultOverheadNsSum
  }
}
