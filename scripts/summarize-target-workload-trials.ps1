param(
  [string]$Label = "local",
  [string]$InputLabel = "",
  [string]$TargetWorkloadName = "cluster_boundary_range",
  [string]$OutputDir = "benchmark_artifacts",
  [string]$TrialOutputDir = ""
)

$ErrorActionPreference = "Stop"

$BenchmarkCsvLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-csv.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

. $BenchmarkCsvLibrary

if ([string]::IsNullOrWhiteSpace($Label)) {
  throw "`-Label` cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($TargetWorkloadName)) {
  throw "`-TargetWorkloadName` cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($InputLabel)) {
  $InputLabel = $Label
}

if ([string]::IsNullOrWhiteSpace($TrialOutputDir)) {
  $TrialOutputDir = Join-Path $OutputDir "low-gap-trials-$InputLabel"
}

if (!(Test-Path -LiteralPath $TrialOutputDir)) {
  throw "trial workload directory was not found: $TrialOutputDir"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$DetailCsvPath = Join-Path $OutputDir "target-workload-trial-details-$Label.csv"
$SummaryCsvPath = Join-Path $OutputDir "target-workload-trial-summary-$Label.csv"
$NotesPath = Join-Path $OutputDir "target-workload-trial-notes-$Label.txt"

$TreeBaselines = @("kd_tree", "r_tree")

function Get-WorkloadMetric {
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

function Get-WorkloadBoolean {
  param(
    [object]$Row,
    [string]$FieldName
  )

  if ($null -eq $Row) {
    return $false
  }

  $RawValue = $Row.$FieldName

  if ([string]::IsNullOrWhiteSpace($RawValue)) {
    return $false
  }

  return $RawValue.ToString().Trim().ToLowerInvariant() -eq "true"
}

function Get-WorkloadRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName,
    [string]$WorkloadName
  )

  return $Rows |
  Where-Object {
    $_.baseline_name -eq $BaselineName -and $_.workload_name -eq $WorkloadName
  } |
  Select-Object -First 1
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

function Get-TrialNumberFromPath {
  param(
    [string]$Path
  )

  $FileName = [System.IO.Path]::GetFileNameWithoutExtension($Path)

  if ($FileName -match "trial-(\d+)$") {
    return [int]$Matches[1]
  }

  return 0
}

function Read-TargetWorkloadTrialRows {
  param(
    [string[]]$WorkloadCsvPaths
  )

  $TrialRows = @()

  foreach ($WorkloadCsvPath in $WorkloadCsvPaths) {
    $TrialNumber = Get-TrialNumberFromPath -Path $WorkloadCsvPath
    $Rows = @(Import-Csv -LiteralPath $WorkloadCsvPath)

    foreach ($BaselineName in $TreeBaselines) {
      $Row = Get-WorkloadRow `
        -Rows $Rows `
        -BaselineName $BaselineName `
        -WorkloadName $TargetWorkloadName

      if ($null -eq $Row) {
        $TrialRows += [PSCustomObject]@{
          Trial                                   = $TrialNumber
          BaselineName                            = $BaselineName
          WorkloadName                            = $TargetWorkloadName
          AverageTimingRatio                      = $null
          BaselineAverageElapsedNs                = $null
          FseAverageElapsedNs                     = $null
          CountOnlyAverageElapsedNs               = $null
          EstimatedOwnedResultOverheadNs          = $null
          CountOnlySpeedupRatio                   = $null
          CountOnlyStatsMatchOwned                = $false
          ReferenceAverageElapsedNs               = $null
          EstimatedOwnedVsReferenceOverheadNs     = $null
          ReferenceResultSpeedupRatio             = $null
          ReferenceStatsMatchCountOnly            = $false
          ReusableOwnedAverageElapsedNs           = $null
          EstimatedFreshVsReusableOwnedOverheadNs = $null
          ReusableOwnedResultSpeedupRatio         = $null
          ReusableOwnedStatsMatchOwned            = $false
          ReusableOwnedVisitedNodes               = $null
          ReusableOwnedRetainedLeaves             = $null
          ReusableOwnedReconstructedRecords       = $null
          ReusableOwnedMatchedRecords             = $null
          ReferenceVisitedNodes                   = $null
          ReferenceRetainedLeaves                 = $null
          ReferenceReconstructedRecords           = $null
          ReferenceMatchedRecords                 = $null
          FseVisitedNodes                         = $null
          FseRetainedLeaves                       = $null
          FseReconstructedRecords                 = $null
          FseMatchedRecords                       = $null
          BaselineEvaluatedRecords                = $null
          BaselineMatchedRecords                  = $null
          CandidateRatio                          = $null
          ReconstructionAvoidanceRatio            = $null
          NodesPerRecord                          = $null
          SourceCsv                               = $WorkloadCsvPath
          Status                                  = "missing target workload"
        }

        continue
      }

      $FseVisitedNodes = Get-WorkloadMetric -Row $Row -FieldName "fse_visited_nodes"
      $FseReconstructedRecords = Get-WorkloadMetric -Row $Row -FieldName "fse_reconstructed_records"

      $TrialRows += [PSCustomObject]@{
        Trial                                   = $TrialNumber
        BaselineName                            = $BaselineName
        WorkloadName                            = $TargetWorkloadName
        AverageTimingRatio                      = Get-WorkloadMetric -Row $Row -FieldName "average_timing_ratio"
        BaselineAverageElapsedNs                = Get-WorkloadMetric -Row $Row -FieldName "baseline_average_elapsed_ns"
        FseAverageElapsedNs                     = Get-WorkloadMetric -Row $Row -FieldName "fse_average_elapsed_ns"
        CountOnlyAverageElapsedNs               = Get-WorkloadMetric -Row $Row -FieldName "count_only_average_elapsed_ns"
        EstimatedOwnedResultOverheadNs          = Get-WorkloadMetric -Row $Row -FieldName "estimated_owned_result_overhead_ns"
        CountOnlySpeedupRatio                   = Get-WorkloadMetric -Row $Row -FieldName "count_only_speedup_ratio"
        CountOnlyStatsMatchOwned                = Get-WorkloadBoolean -Row $Row -FieldName "count_only_stats_match_owned"
        ReferenceAverageElapsedNs               = Get-WorkloadMetric -Row $Row -FieldName "reference_average_elapsed_ns"
        EstimatedOwnedVsReferenceOverheadNs     = Get-WorkloadMetric -Row $Row -FieldName "estimated_owned_vs_reference_overhead_ns"
        ReferenceResultSpeedupRatio             = Get-WorkloadMetric -Row $Row -FieldName "reference_result_speedup_ratio"
        ReferenceStatsMatchCountOnly            = Get-WorkloadBoolean -Row $Row -FieldName "reference_stats_match_count_only"
        ReusableOwnedAverageElapsedNs           = Get-WorkloadMetric -Row $Row -FieldName "reusable_owned_average_elapsed_ns"
        EstimatedFreshVsReusableOwnedOverheadNs = Get-WorkloadMetric -Row $Row -FieldName "estimated_fresh_vs_reusable_owned_overhead_ns"
        ReusableOwnedResultSpeedupRatio         = Get-WorkloadMetric -Row $Row -FieldName "reusable_owned_result_speedup_ratio"
        ReusableOwnedStatsMatchOwned            = Get-WorkloadBoolean -Row $Row -FieldName "reusable_owned_stats_match_owned"
        ReusableOwnedVisitedNodes               = Get-WorkloadMetric -Row $Row -FieldName "reusable_owned_visited_nodes"
        ReusableOwnedRetainedLeaves             = Get-WorkloadMetric -Row $Row -FieldName "reusable_owned_retained_leaves"
        ReusableOwnedReconstructedRecords       = Get-WorkloadMetric -Row $Row -FieldName "reusable_owned_reconstructed_records"
        ReusableOwnedMatchedRecords             = Get-WorkloadMetric -Row $Row -FieldName "reusable_owned_matched_records"
        ReferenceVisitedNodes                   = Get-WorkloadMetric -Row $Row -FieldName "reference_visited_nodes"
        ReferenceRetainedLeaves                 = Get-WorkloadMetric -Row $Row -FieldName "reference_retained_leaves"
        ReferenceReconstructedRecords           = Get-WorkloadMetric -Row $Row -FieldName "reference_reconstructed_records"
        ReferenceMatchedRecords                 = Get-WorkloadMetric -Row $Row -FieldName "reference_matched_records"
        FseVisitedNodes                         = $FseVisitedNodes
        FseRetainedLeaves                       = Get-WorkloadMetric -Row $Row -FieldName "fse_retained_leaves"
        FseReconstructedRecords                 = $FseReconstructedRecords
        FseMatchedRecords                       = Get-WorkloadMetric -Row $Row -FieldName "fse_matched_records"
        BaselineEvaluatedRecords                = Get-WorkloadMetric -Row $Row -FieldName "baseline_evaluated_records"
        BaselineMatchedRecords                  = Get-WorkloadMetric -Row $Row -FieldName "baseline_matched_records"
        CandidateRatio                          = Get-WorkloadMetric -Row $Row -FieldName "candidate_ratio"
        ReconstructionAvoidanceRatio            = Get-WorkloadMetric -Row $Row -FieldName "reconstruction_avoidance_ratio"
        NodesPerRecord                          = Get-RatioOrNull -Numerator $FseVisitedNodes -Denominator $FseReconstructedRecords
        SourceCsv                               = $WorkloadCsvPath
        Status                                  = "loaded"
      }
    }
  }

  return $TrialRows
}

function New-TargetSummaryRow {
  param(
    [string]$BaselineName,
    [object[]]$TrialRows
  )

  $BaselineRows = @($TrialRows | Where-Object {
      $_.BaselineName -eq $BaselineName -and $_.Status -eq "loaded"
    })

  $TimingValues = @($BaselineRows | Where-Object {
      $null -ne $_.AverageTimingRatio
    } | ForEach-Object {
      [double]$_.AverageTimingRatio
    })
  $BaselineAverageValues = @($BaselineRows | Where-Object {
      $null -ne $_.BaselineAverageElapsedNs
    } | ForEach-Object {
      [double]$_.BaselineAverageElapsedNs
    })
  $FseAverageValues = @($BaselineRows | Where-Object {
      $null -ne $_.FseAverageElapsedNs
    } | ForEach-Object {
      [double]$_.FseAverageElapsedNs
    })
  $CountOnlyAverageValues = @($BaselineRows | Where-Object {
      $null -ne $_.CountOnlyAverageElapsedNs
    } | ForEach-Object {
      [double]$_.CountOnlyAverageElapsedNs
    })
  $OwnedResultOverheadValues = @($BaselineRows | Where-Object {
      $null -ne $_.EstimatedOwnedResultOverheadNs
    } | ForEach-Object {
      [double]$_.EstimatedOwnedResultOverheadNs
    })
  $CountOnlySpeedupValues = @($BaselineRows | Where-Object {
      $null -ne $_.CountOnlySpeedupRatio
    } | ForEach-Object {
      [double]$_.CountOnlySpeedupRatio
    })
  $StatsMatchRows = @($BaselineRows | Where-Object {
      $_.CountOnlyStatsMatchOwned
    })
  $ReferenceAverageValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReferenceAverageElapsedNs
    } | ForEach-Object {
      [double]$_.ReferenceAverageElapsedNs
    })
  $OwnedVsReferenceOverheadValues = @($BaselineRows | Where-Object {
      $null -ne $_.EstimatedOwnedVsReferenceOverheadNs
    } | ForEach-Object {
      [double]$_.EstimatedOwnedVsReferenceOverheadNs
    })
  $ReferenceSpeedupValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReferenceResultSpeedupRatio
    } | ForEach-Object {
      [double]$_.ReferenceResultSpeedupRatio
    })
  $ReferenceStatsMatchRows = @($BaselineRows | Where-Object {
      $_.ReferenceStatsMatchCountOnly
    })
  $ReusableOwnedAverageValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReusableOwnedAverageElapsedNs
    } | ForEach-Object {
      [double]$_.ReusableOwnedAverageElapsedNs
    })
  $FreshVsReusableOwnedOverheadValues = @($BaselineRows | Where-Object {
      $null -ne $_.EstimatedFreshVsReusableOwnedOverheadNs
    } | ForEach-Object {
      [double]$_.EstimatedFreshVsReusableOwnedOverheadNs
    })
  $ReusableOwnedSpeedupValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReusableOwnedResultSpeedupRatio
    } | ForEach-Object {
      [double]$_.ReusableOwnedResultSpeedupRatio
    })
  $ReusableOwnedStatsMatchRows = @($BaselineRows | Where-Object {
      $_.ReusableOwnedStatsMatchOwned
    })
  $ReusableOwnedVisitedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReusableOwnedVisitedNodes
    } | ForEach-Object {
      [double]$_.ReusableOwnedVisitedNodes
    })
  $ReusableOwnedRetainedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReusableOwnedRetainedLeaves
    } | ForEach-Object {
      [double]$_.ReusableOwnedRetainedLeaves
    })
  $ReusableOwnedReconstructedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReusableOwnedReconstructedRecords
    } | ForEach-Object {
      [double]$_.ReusableOwnedReconstructedRecords
    })
  $ReusableOwnedMatchedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReusableOwnedMatchedRecords
    } | ForEach-Object {
      [double]$_.ReusableOwnedMatchedRecords
    })
  $ReferenceVisitedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReferenceVisitedNodes
    } | ForEach-Object {
      [double]$_.ReferenceVisitedNodes
    })
  $ReferenceRetainedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReferenceRetainedLeaves
    } | ForEach-Object {
      [double]$_.ReferenceRetainedLeaves
    })
  $ReferenceReconstructedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReferenceReconstructedRecords
    } | ForEach-Object {
      [double]$_.ReferenceReconstructedRecords
    })
  $ReferenceMatchedValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReferenceMatchedRecords
    } | ForEach-Object {
      [double]$_.ReferenceMatchedRecords
    })
  $VisitedValues = @($BaselineRows | Where-Object {
      $null -ne $_.FseVisitedNodes
    } | ForEach-Object {
      [double]$_.FseVisitedNodes
    })
  $RetainedValues = @($BaselineRows | Where-Object {
      $null -ne $_.FseRetainedLeaves
    } | ForEach-Object {
      [double]$_.FseRetainedLeaves
    })
  $ReconstructedValues = @($BaselineRows | Where-Object {
      $null -ne $_.FseReconstructedRecords
    } | ForEach-Object {
      [double]$_.FseReconstructedRecords
    })
  $MatchedValues = @($BaselineRows | Where-Object {
      $null -ne $_.FseMatchedRecords
    } | ForEach-Object {
      [double]$_.FseMatchedRecords
    })
  $BaselineEvaluatedValues = @($BaselineRows | Where-Object {
      $null -ne $_.BaselineEvaluatedRecords
    } | ForEach-Object {
      [double]$_.BaselineEvaluatedRecords
    })
  $BaselineMatchedValues = @($BaselineRows | Where-Object {
      $null -ne $_.BaselineMatchedRecords
    } | ForEach-Object {
      [double]$_.BaselineMatchedRecords
    })
  $CandidateValues = @($BaselineRows | Where-Object {
      $null -ne $_.CandidateRatio
    } | ForEach-Object {
      [double]$_.CandidateRatio
    })
  $AvoidanceValues = @($BaselineRows | Where-Object {
      $null -ne $_.ReconstructionAvoidanceRatio
    } | ForEach-Object {
      [double]$_.ReconstructionAvoidanceRatio
    })
  $NodesPerRecordValues = @($BaselineRows | Where-Object {
      $null -ne $_.NodesPerRecord
    } | ForEach-Object {
      [double]$_.NodesPerRecord
    })

  $MeanTiming = Get-Mean -Values $TimingValues
  $MinTiming = Get-Minimum -Values $TimingValues
  $MaxTiming = Get-Maximum -Values $TimingValues
  $TimingRange = $null

  if ($null -ne $MinTiming -and $null -ne $MaxTiming) {
    $TimingRange = $MaxTiming - $MinTiming
  }

  return [PSCustomObject]@{
    BaselineName                          = $BaselineName
    WorkloadName                          = $TargetWorkloadName
    TrialCount                            = $TimingValues.Count
    MeanTimingRatio                       = $MeanTiming
    MinTimingRatio                        = $MinTiming
    MaxTimingRatio                        = $MaxTiming
    RangeTimingRatio                      = $TimingRange
    SampleStdDevTimingRatio               = Get-SampleStdDev -Values $TimingValues -Mean $MeanTiming
    MeanBaselineAverageElapsedNs          = Get-Mean -Values $BaselineAverageValues
    MeanFseAverageElapsedNs               = Get-Mean -Values $FseAverageValues
    MeanCountOnlyAverageElapsedNs         = Get-Mean -Values $CountOnlyAverageValues
    MeanOwnedResultOverheadNs             = Get-Mean -Values $OwnedResultOverheadValues
    MeanCountOnlySpeedupRatio             = Get-Mean -Values $CountOnlySpeedupValues
    CountOnlyStatsAgreeCount              = $StatsMatchRows.Count
    AllCountOnlyStatsMatchOwned           = $StatsMatchRows.Count -eq $BaselineRows.Count
    MeanReferenceAverageElapsedNs         = Get-Mean -Values $ReferenceAverageValues
    MeanOwnedVsReferenceOverheadNs        = Get-Mean -Values $OwnedVsReferenceOverheadValues
    MeanReferenceResultSpeedupRatio       = Get-Mean -Values $ReferenceSpeedupValues
    ReferenceStatsAgreeCount              = $ReferenceStatsMatchRows.Count
    AllReferenceStatsMatchCountOnly       = $ReferenceStatsMatchRows.Count -eq $BaselineRows.Count
    MeanReusableOwnedAverageElapsedNs     = Get-Mean -Values $ReusableOwnedAverageValues
    MeanFreshVsReusableOwnedOverheadNs    = Get-Mean -Values $FreshVsReusableOwnedOverheadValues
    MeanReusableOwnedResultSpeedupRatio   = Get-Mean -Values $ReusableOwnedSpeedupValues
    ReusableOwnedStatsAgreeCount          = $ReusableOwnedStatsMatchRows.Count
    AllReusableOwnedStatsMatchOwned       = $ReusableOwnedStatsMatchRows.Count -eq $BaselineRows.Count
    MeanReusableOwnedVisitedNodes         = Get-Mean -Values $ReusableOwnedVisitedValues
    MeanReusableOwnedRetainedLeaves       = Get-Mean -Values $ReusableOwnedRetainedValues
    MeanReusableOwnedReconstructedRecords = Get-Mean -Values $ReusableOwnedReconstructedValues
    MeanReusableOwnedMatchedRecords       = Get-Mean -Values $ReusableOwnedMatchedValues
    MeanReferenceVisitedNodes             = Get-Mean -Values $ReferenceVisitedValues
    MeanReferenceRetainedLeaves           = Get-Mean -Values $ReferenceRetainedValues
    MeanReferenceReconstructedRecords     = Get-Mean -Values $ReferenceReconstructedValues
    MeanReferenceMatchedRecords           = Get-Mean -Values $ReferenceMatchedValues
    MeanFseVisitedNodes                   = Get-Mean -Values $VisitedValues
    MeanFseRetainedLeaves                 = Get-Mean -Values $RetainedValues
    MeanFseReconstructedRecords           = Get-Mean -Values $ReconstructedValues
    MeanFseMatchedRecords                 = Get-Mean -Values $MatchedValues
    MeanBaselineEvaluatedRecords          = Get-Mean -Values $BaselineEvaluatedValues
    MeanBaselineMatchedRecords            = Get-Mean -Values $BaselineMatchedValues
    MeanCandidateRatio                    = Get-Mean -Values $CandidateValues
    MeanReconstructionAvoidanceRatio      = Get-Mean -Values $AvoidanceValues
    MeanNodesPerRecord                    = Get-Mean -Values $NodesPerRecordValues
  }
}

$WorkloadCsvPaths = @(
  Get-ChildItem -LiteralPath $TrialOutputDir -Filter "workloads-$InputLabel-trial-*.csv" |
  Sort-Object Name |
  ForEach-Object { $_.FullName }
)

if ($WorkloadCsvPaths.Count -eq 0) {
  throw "no workload trial CSV files were found in $TrialOutputDir for input label $InputLabel"
}

$TrialRows = Read-TargetWorkloadTrialRows -WorkloadCsvPaths $WorkloadCsvPaths

$DetailRows = @()

foreach ($Row in $TrialRows) {
  $DetailRows += , @(
    $Row.Trial,
    $Row.BaselineName,
    $Row.WorkloadName,
    $(Format-InvariantDouble -Value $Row.AverageTimingRatio),
    $(Format-InvariantDouble -Value $Row.BaselineAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.FseAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.CountOnlyAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.EstimatedOwnedResultOverheadNs),
    $(Format-InvariantDouble -Value $Row.CountOnlySpeedupRatio),
    $Row.CountOnlyStatsMatchOwned.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Row.ReferenceAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.EstimatedOwnedVsReferenceOverheadNs),
    $(Format-InvariantDouble -Value $Row.ReferenceResultSpeedupRatio),
    $Row.ReferenceStatsMatchCountOnly.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedAverageElapsedNs),
    $(Format-InvariantDouble -Value $Row.EstimatedFreshVsReusableOwnedOverheadNs),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedResultSpeedupRatio),
    $Row.ReusableOwnedStatsMatchOwned.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedVisitedNodes),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedRetainedLeaves),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedReconstructedRecords),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedMatchedRecords),
    $(Format-InvariantDouble -Value $Row.ReferenceVisitedNodes),
    $(Format-InvariantDouble -Value $Row.ReferenceRetainedLeaves),
    $(Format-InvariantDouble -Value $Row.ReferenceReconstructedRecords),
    $(Format-InvariantDouble -Value $Row.ReferenceMatchedRecords),
    $(Format-InvariantDouble -Value $Row.FseVisitedNodes),
    $(Format-InvariantDouble -Value $Row.FseRetainedLeaves),
    $(Format-InvariantDouble -Value $Row.FseReconstructedRecords),
    $(Format-InvariantDouble -Value $Row.FseMatchedRecords),
    $(Format-InvariantDouble -Value $Row.BaselineEvaluatedRecords),
    $(Format-InvariantDouble -Value $Row.BaselineMatchedRecords),
    $(Format-InvariantDouble -Value $Row.CandidateRatio),
    $(Format-InvariantDouble -Value $Row.ReconstructionAvoidanceRatio),
    $(Format-InvariantDouble -Value $Row.NodesPerRecord),
    $Row.Status,
    $Row.SourceCsv
  )
}

Write-CsvDocument `
  -Path $DetailCsvPath `
  -Header @(
  "trial",
  "baseline_name",
  "workload_name",
  "average_timing_ratio",
  "baseline_average_elapsed_ns",
  "fse_average_elapsed_ns",
  "count_only_average_elapsed_ns",
  "estimated_owned_result_overhead_ns",
  "count_only_speedup_ratio",
  "count_only_stats_match_owned",
  "reference_average_elapsed_ns",
  "estimated_owned_vs_reference_overhead_ns",
  "reference_result_speedup_ratio",
  "reference_stats_match_count_only",
  "reusable_owned_average_elapsed_ns",
  "estimated_fresh_vs_reusable_owned_overhead_ns",
  "reusable_owned_result_speedup_ratio",
  "reusable_owned_stats_match_owned",
  "reusable_owned_visited_nodes",
  "reusable_owned_retained_leaves",
  "reusable_owned_reconstructed_records",
  "reusable_owned_matched_records",
  "reference_visited_nodes",
  "reference_retained_leaves",
  "reference_reconstructed_records",
  "reference_matched_records",
  "fse_visited_nodes",
  "fse_retained_leaves",
  "fse_reconstructed_records",
  "fse_matched_records",
  "baseline_evaluated_records",
  "baseline_matched_records",
  "candidate_ratio",
  "reconstruction_avoidance_ratio",
  "nodes_per_record",
  "status",
  "source_csv"
) `
  -Rows $DetailRows

$SummaryObjects = @()

foreach ($BaselineName in $TreeBaselines) {
  $SummaryObjects += New-TargetSummaryRow `
    -BaselineName $BaselineName `
    -TrialRows $TrialRows
}

$SummaryRows = @()

foreach ($Summary in $SummaryObjects) {
  $SummaryRows += , @(
    $Summary.BaselineName,
    $Summary.WorkloadName,
    $Summary.TrialCount,
    $(Format-InvariantDouble -Value $Summary.MeanTimingRatio),
    $(Format-InvariantDouble -Value $Summary.MinTimingRatio),
    $(Format-InvariantDouble -Value $Summary.MaxTimingRatio),
    $(Format-InvariantDouble -Value $Summary.RangeTimingRatio),
    $(Format-InvariantDouble -Value $Summary.SampleStdDevTimingRatio),
    $(Format-InvariantDouble -Value $Summary.MeanBaselineAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanFseAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanCountOnlyAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanOwnedResultOverheadNs),
    $(Format-InvariantDouble -Value $Summary.MeanCountOnlySpeedupRatio),
    $Summary.CountOnlyStatsAgreeCount,
    $Summary.AllCountOnlyStatsMatchOwned.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanOwnedVsReferenceOverheadNs),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceResultSpeedupRatio),
    $Summary.ReferenceStatsAgreeCount,
    $Summary.AllReferenceStatsMatchCountOnly.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedAverageElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanFreshVsReusableOwnedOverheadNs),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedResultSpeedupRatio),
    $Summary.ReusableOwnedStatsAgreeCount,
    $Summary.AllReusableOwnedStatsMatchOwned.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedVisitedNodes),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedRetainedLeaves),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedReconstructedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceVisitedNodes),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceRetainedLeaves),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceReconstructedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanFseVisitedNodes),
    $(Format-InvariantDouble -Value $Summary.MeanFseRetainedLeaves),
    $(Format-InvariantDouble -Value $Summary.MeanFseReconstructedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanFseMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanBaselineEvaluatedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanBaselineMatchedRecords),
    $(Format-InvariantDouble -Value $Summary.MeanCandidateRatio),
    $(Format-InvariantDouble -Value $Summary.MeanReconstructionAvoidanceRatio),
    $(Format-InvariantDouble -Value $Summary.MeanNodesPerRecord)
  )
}

Write-CsvDocument `
  -Path $SummaryCsvPath `
  -Header @(
  "baseline_name",
  "workload_name",
  "trial_count",
  "mean_timing_ratio",
  "min_timing_ratio",
  "max_timing_ratio",
  "range_timing_ratio",
  "sample_stddev_timing_ratio",
  "mean_baseline_average_elapsed_ns",
  "mean_fse_average_elapsed_ns",
  "mean_count_only_average_elapsed_ns",
  "mean_owned_result_overhead_ns",
  "mean_count_only_speedup_ratio",
  "count_only_stats_agree_count",
  "all_count_only_stats_match_owned",
  "mean_reference_average_elapsed_ns",
  "mean_owned_vs_reference_overhead_ns",
  "mean_reference_result_speedup_ratio",
  "reference_stats_agree_count",
  "all_reference_stats_match_count_only",
  "mean_reusable_owned_average_elapsed_ns",
  "mean_fresh_vs_reusable_owned_overhead_ns",
  "mean_reusable_owned_result_speedup_ratio",
  "reusable_owned_stats_agree_count",
  "all_reusable_owned_stats_match_owned",
  "mean_reusable_owned_visited_nodes",
  "mean_reusable_owned_retained_leaves",
  "mean_reusable_owned_reconstructed_records",
  "mean_reusable_owned_matched_records",
  "mean_reference_visited_nodes",
  "mean_reference_retained_leaves",
  "mean_reference_reconstructed_records",
  "mean_reference_matched_records",
  "mean_fse_visited_nodes",
  "mean_fse_retained_leaves",
  "mean_fse_reconstructed_records",
  "mean_fse_matched_records",
  "mean_baseline_evaluated_records",
  "mean_baseline_matched_records",
  "mean_candidate_ratio",
  "mean_reconstruction_avoidance_ratio",
  "mean_nodes_per_record"
) `
  -Rows $SummaryRows

Set-Utf8Text -Path $NotesPath -Text "Target workload trial summary`r`n"
Add-Utf8Text -Path $NotesPath -Text "=============================`r`n`r`n"
Add-Utf8Text -Path $NotesPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $NotesPath -Text "Input label: $InputLabel`r`n"
Add-Utf8Text -Path $NotesPath -Text "Target workload: $TargetWorkloadName`r`n"
Add-Utf8Text -Path $NotesPath -Text "Trial workload directory: $TrialOutputDir`r`n"
Add-Utf8Text -Path $NotesPath -Text "Workload CSV files found: $($WorkloadCsvPaths.Count)`r`n"
Add-Utf8Text -Path $NotesPath -Text "Timing ratio meaning: baseline elapsed / FSE elapsed; higher is better for FSE.`r`n"

foreach ($Summary in $SummaryObjects) {
  Add-Utf8Text -Path $NotesPath -Text "`r`n$($Summary.BaselineName)`r`n"
  Add-Utf8Text -Path $NotesPath -Text ("-" * $Summary.BaselineName.Length)
  Add-Utf8Text -Path $NotesPath -Text "`r`n"
  Add-Utf8Text -Path $NotesPath -Text "trials loaded: $($Summary.TrialCount)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean timing ratio: $(Format-InvariantDouble -Value $Summary.MeanTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "min timing ratio: $(Format-InvariantDouble -Value $Summary.MinTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "max timing ratio: $(Format-InvariantDouble -Value $Summary.MaxTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "range timing ratio: $(Format-InvariantDouble -Value $Summary.RangeTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "sample stddev timing ratio: $(Format-InvariantDouble -Value $Summary.SampleStdDevTimingRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean baseline average ns: $(Format-InvariantDouble -Value $Summary.MeanBaselineAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean FSE owned-result average ns: $(Format-InvariantDouble -Value $Summary.MeanFseAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean count-only average ns: $(Format-InvariantDouble -Value $Summary.MeanCountOnlyAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean estimated owned-result overhead ns: $(Format-InvariantDouble -Value $Summary.MeanOwnedResultOverheadNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean count-only speedup ratio: $(Format-InvariantDouble -Value $Summary.MeanCountOnlySpeedupRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "count-only stats agree count: $($Summary.CountOnlyStatsAgreeCount)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "all count-only stats match owned: $($Summary.AllCountOnlyStatsMatchOwned.ToString().ToLowerInvariant())`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference-result average ns: $(Format-InvariantDouble -Value $Summary.MeanReferenceAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean owned-vs-reference overhead ns: $(Format-InvariantDouble -Value $Summary.MeanOwnedVsReferenceOverheadNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference-result speedup ratio: $(Format-InvariantDouble -Value $Summary.MeanReferenceResultSpeedupRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "reference stats agree count: $($Summary.ReferenceStatsAgreeCount)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "all reference stats match count-only: $($Summary.AllReferenceStatsMatchCountOnly.ToString().ToLowerInvariant())`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned-result average ns: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedAverageElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean fresh-vs-reusable owned overhead ns: $(Format-InvariantDouble -Value $Summary.MeanFreshVsReusableOwnedOverheadNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned-result speedup ratio: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedResultSpeedupRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "reusable owned stats agree count: $($Summary.ReusableOwnedStatsAgreeCount)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "all reusable owned stats match owned: $($Summary.AllReusableOwnedStatsMatchOwned.ToString().ToLowerInvariant())`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned visited nodes: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedVisitedNodes)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned retained leaves: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedRetainedLeaves)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned reconstructed records: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedReconstructedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned matched records: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedMatchedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference visited nodes: $(Format-InvariantDouble -Value $Summary.MeanReferenceVisitedNodes)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference retained leaves: $(Format-InvariantDouble -Value $Summary.MeanReferenceRetainedLeaves)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference reconstructed records: $(Format-InvariantDouble -Value $Summary.MeanReferenceReconstructedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference matched records: $(Format-InvariantDouble -Value $Summary.MeanReferenceMatchedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean FSE visited nodes: $(Format-InvariantDouble -Value $Summary.MeanFseVisitedNodes)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean FSE retained leaves: $(Format-InvariantDouble -Value $Summary.MeanFseRetainedLeaves)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean FSE reconstructed records: $(Format-InvariantDouble -Value $Summary.MeanFseReconstructedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean FSE matched records: $(Format-InvariantDouble -Value $Summary.MeanFseMatchedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean baseline evaluated records: $(Format-InvariantDouble -Value $Summary.MeanBaselineEvaluatedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean candidate ratio: $(Format-InvariantDouble -Value $Summary.MeanCandidateRatio)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean nodes per record: $(Format-InvariantDouble -Value $Summary.MeanNodesPerRecord)`r`n"
}

Add-Utf8Text -Path $NotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $NotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $NotesPath -Text "Use this target summary when a boundary optimization improves the selected target workload enough that it may stop being the weakest workload.`r`n"
Add-Utf8Text -Path $NotesPath -Text "The weakest-workload summary answers what lost most often.`r`n"
Add-Utf8Text -Path $NotesPath -Text "This target-workload summary answers what happened to the workload we were intentionally targeting.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Owned-result timing answers materialized row-return performance using the normal fresh result Vec path.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Reusable owned-result timing answers materialized row-return performance when the caller reuses the outer result Vec allocation.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Reference-result timing answers exact leaf/row-reference performance without owned Vector materialization.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Count-only timing answers exact-cardinality performance and must only be interpreted when all count-only stats match owned-result stats.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Reusable owned-result timing must only be interpreted when all reusable owned stats match owned-result stats.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Reference-result timing must only be interpreted when all reference stats match count-only stats.`r`n"

Write-Host ""
Write-Host "Target workload trial artifacts written:"
Write-Host "  $DetailCsvPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $NotesPath"