<#
    Repeated low-selectivity gap trial helpers.

    These functions keep CSV loading, trial-row projection, and summary-row
    construction separate from the runner that executes cargo trials.
#>

function Get-CsvLoadStatus {
  param(
    [string]$Path
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return [PSCustomObject]@{
      Status       = "not provided"
      Loaded       = $false
      ResolvedPath = ""
      Rows         = @()
    }
  }

  if (!(Test-Path -LiteralPath $Path)) {
    return [PSCustomObject]@{
      Status       = "path not found"
      Loaded       = $false
      ResolvedPath = ""
      Rows         = @()
    }
  }

  $ResolvedPath = (Resolve-Path -LiteralPath $Path).Path
  $Rows = @(Import-Csv -LiteralPath $ResolvedPath)

  return [PSCustomObject]@{
    Status       = "loaded"
    Loaded       = $true
    ResolvedPath = $ResolvedPath
    Rows         = $Rows
  }
}

function Get-MostFrequentText {
  param(
    [object[]]$Values
  )

  if ($Values.Count -eq 0) {
    return "unavailable"
  }

  $BestValue = "unavailable"
  $BestCount = 0
  $Counts = @{}

  foreach ($Value in $Values) {
    if ($null -eq $Value) {
      continue
    }

    $Text = $Value.ToString()

    if ([string]::IsNullOrWhiteSpace($Text)) {
      continue
    }

    if (!$Counts.ContainsKey($Text)) {
      $Counts[$Text] = 0
    }

    $Counts[$Text] += 1

    if ($Counts[$Text] -gt $BestCount) {
      $BestValue = $Text
      $BestCount = $Counts[$Text]
    }
  }

  return $BestValue
}

function Read-LowGapTrialRows {
  param(
    [int]$TrialNumber,
    [string]$LowGapCsvPath,
    [string[]]$TreeBaselines
  )

  $Rows = @(Import-Csv -LiteralPath $LowGapCsvPath)
  $TrialRows = @()

  foreach ($BaselineName in $TreeBaselines) {
    $Row = Get-LowGapRow -Rows $Rows -BaselineName $BaselineName

    if ($null -eq $Row) {
      $TrialRows += [PSCustomObject]@{
        Trial                     = $TrialNumber
        BaselineName              = $BaselineName
        LowMeanTimingRatio        = $null
        LowWeightedCandidateRatio = $null
        LowWeightedAvoidanceRatio = $null
        SourceCsv                 = $LowGapCsvPath
        Status                    = "missing row"
      }

      continue
    }

    $TrialRows += [PSCustomObject]@{
      Trial                     = $TrialNumber
      BaselineName              = $BaselineName
      LowMeanTimingRatio        = Get-LowGapMetric -Row $Row -FieldName "low_mean_timing_ratio"
      LowWeightedCandidateRatio = Get-LowGapMetric -Row $Row -FieldName "low_weighted_candidate_ratio"
      LowWeightedAvoidanceRatio = Get-LowGapMetric -Row $Row -FieldName "low_weighted_reconstruction_avoidance_ratio"
      SourceCsv                 = $LowGapCsvPath
      Status                    = "loaded"
    }
  }

  return $TrialRows
}

function Read-LowGapWeakestWorkloadRows {
  param(
    [int]$TrialNumber,
    [string]$WorkloadsCsvPath,
    [string[]]$TreeBaselines
  )

  $Rows = @(Import-Csv -LiteralPath $WorkloadsCsvPath)
  $TrialRows = @()

  foreach ($BaselineName in $TreeBaselines) {
    $WeakestRow = $null
    $WeakestTiming = $null

    foreach ($Row in $Rows) {
      if ($Row.baseline_name -ne $BaselineName) {
        continue
      }

      $CandidateRatio = Get-WorkloadMetric -Row $Row -FieldName "candidate_ratio"

      if ($null -eq $CandidateRatio) {
        continue
      }

      if ($CandidateRatio -le 0.0 -or $CandidateRatio -ge 1.0) {
        continue
      }

      $AverageTimingRatio = Get-WorkloadMetric -Row $Row -FieldName "average_timing_ratio"

      if ($null -eq $AverageTimingRatio) {
        continue
      }

      if ($null -eq $WeakestTiming -or $AverageTimingRatio -lt $WeakestTiming) {
        $WeakestTiming = $AverageTimingRatio
        $WeakestRow = $Row
      }
    }

    if ($null -eq $WeakestRow) {
      $TrialRows += [PSCustomObject]@{
        Trial                    = $TrialNumber
        BaselineName             = $BaselineName
        WorkloadName             = "unavailable"
        AverageTimingRatio       = $null
        BaselineAverageElapsedNs = $null
        FseAverageElapsedNs      = $null
        FseVisitedNodes          = $null
        FseRetainedLeaves        = $null
        FseReconstructedRecords  = $null
        FseMatchedRecords        = $null
        BaselineEvaluatedRecords = $null
        CandidateRatio           = $null
        NodesPerRecord           = $null
        SourceCsv                = $WorkloadsCsvPath
        Status                   = "missing low workload"
      }

      continue
    }

    $FseVisitedNodes = Get-WorkloadMetric -Row $WeakestRow -FieldName "fse_visited_nodes"
    $FseReconstructedRecords = Get-WorkloadMetric -Row $WeakestRow -FieldName "fse_reconstructed_records"

    $TrialRows += [PSCustomObject]@{
      Trial                    = $TrialNumber
      BaselineName             = $BaselineName
      WorkloadName             = Get-TextField -Row $WeakestRow -FieldName "workload_name"
      AverageTimingRatio       = Get-WorkloadMetric -Row $WeakestRow -FieldName "average_timing_ratio"
      BaselineAverageElapsedNs = Get-WorkloadMetric -Row $WeakestRow -FieldName "baseline_average_elapsed_ns"
      FseAverageElapsedNs      = Get-WorkloadMetric -Row $WeakestRow -FieldName "fse_average_elapsed_ns"
      FseVisitedNodes          = $FseVisitedNodes
      FseRetainedLeaves        = Get-WorkloadMetric -Row $WeakestRow -FieldName "fse_retained_leaves"
      FseReconstructedRecords  = $FseReconstructedRecords
      FseMatchedRecords        = Get-WorkloadMetric -Row $WeakestRow -FieldName "fse_matched_records"
      BaselineEvaluatedRecords = Get-WorkloadMetric -Row $WeakestRow -FieldName "baseline_evaluated_records"
      CandidateRatio           = Get-WorkloadMetric -Row $WeakestRow -FieldName "candidate_ratio"
      NodesPerRecord           = Get-RatioOrNull -Numerator $FseVisitedNodes -Denominator $FseReconstructedRecords
      SourceCsv                = $WorkloadsCsvPath
      Status                   = "loaded"
    }
  }

  return $TrialRows
}

function New-LowGapTrialSummaryRow {
  param(
    [string]$BaselineName,
    [object[]]$TrialRows,
    [object[]]$PreviousRows,
    [double]$LowGapNoiseThreshold
  )

  $BaselineRows = @($TrialRows | Where-Object {
      $_.BaselineName -eq $BaselineName -and $null -ne $_.LowMeanTimingRatio
    })

  $TimingValues = @($BaselineRows | ForEach-Object { [double]$_.LowMeanTimingRatio })
  $CandidateValues = @($BaselineRows | Where-Object {
      $null -ne $_.LowWeightedCandidateRatio
    } | ForEach-Object {
      [double]$_.LowWeightedCandidateRatio
    })
  $AvoidanceValues = @($BaselineRows | Where-Object {
      $null -ne $_.LowWeightedAvoidanceRatio
    } | ForEach-Object {
      [double]$_.LowWeightedAvoidanceRatio
    })

  $MeanTiming = Get-Mean -Values $TimingValues
  $MinTiming = Get-Minimum -Values $TimingValues
  $MaxTiming = Get-Maximum -Values $TimingValues
  $TimingRange = $null

  if ($null -ne $MinTiming -and $null -ne $MaxTiming) {
    $TimingRange = $MaxTiming - $MinTiming
  }

  $StdDevTiming = Get-SampleStdDev -Values $TimingValues -Mean $MeanTiming
  $MeanCandidate = Get-Mean -Values $CandidateValues
  $MeanAvoidance = Get-Mean -Values $AvoidanceValues

  $PreviousRow = Get-LowGapRow -Rows $PreviousRows -BaselineName $BaselineName
  $PreviousMeanTiming = Get-LowGapMetric -Row $PreviousRow -FieldName "low_mean_timing_ratio"

  $DeltaFromPrevious = $null

  if ($null -ne $MeanTiming -and $null -ne $PreviousMeanTiming) {
    $DeltaFromPrevious = $MeanTiming - $PreviousMeanTiming
  }

  $Classification = Get-LowGapClassification `
    -Delta $DeltaFromPrevious `
    -Threshold $LowGapNoiseThreshold

  return [PSCustomObject]@{
    BaselineName                   = $BaselineName
    TrialCount                     = $TimingValues.Count
    MeanLowMeanTimingRatio         = $MeanTiming
    MinLowMeanTimingRatio          = $MinTiming
    MaxLowMeanTimingRatio          = $MaxTiming
    RangeLowMeanTimingRatio        = $TimingRange
    SampleStdDevLowMeanTimingRatio = $StdDevTiming
    PreviousLowMeanTimingRatio     = $PreviousMeanTiming
    DeltaFromPrevious              = $DeltaFromPrevious
    Classification                 = $Classification
    MeanLowWeightedCandidateRatio  = $MeanCandidate
    MeanLowWeightedAvoidanceRatio  = $MeanAvoidance
  }
}

function New-LowGapWeakestWorkloadSummaryRow {
  param(
    [string]$BaselineName,
    [object[]]$WorkloadRows
  )

  $BaselineRows = @($WorkloadRows | Where-Object {
      $_.BaselineName -eq $BaselineName -and $_.Status -eq "loaded"
    })

  $WorkloadNames = @($BaselineRows | ForEach-Object { $_.WorkloadName })
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
  $BaselineRecordValues = @($BaselineRows | Where-Object {
      $null -ne $_.BaselineEvaluatedRecords
    } | ForEach-Object {
      [double]$_.BaselineEvaluatedRecords
    })
  $CandidateValues = @($BaselineRows | Where-Object {
      $null -ne $_.CandidateRatio
    } | ForEach-Object {
      [double]$_.CandidateRatio
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
    BaselineName                   = $BaselineName
    TrialCount                     = $TimingValues.Count
    MostFrequentWeakestWorkload    = Get-MostFrequentText -Values $WorkloadNames
    WeakestWorkloadSequence        = ($WorkloadNames -join ";")
    MeanWeakestTimingRatio         = $MeanTiming
    MinWeakestTimingRatio          = $MinTiming
    MaxWeakestTimingRatio          = $MaxTiming
    RangeWeakestTimingRatio        = $TimingRange
    SampleStdDevWeakestTimingRatio = Get-SampleStdDev -Values $TimingValues -Mean $MeanTiming
    MeanBaselineAverageElapsedNs   = Get-Mean -Values $BaselineAverageValues
    MeanFseAverageElapsedNs        = Get-Mean -Values $FseAverageValues
    MeanFseVisitedNodes            = Get-Mean -Values $VisitedValues
    MeanFseRetainedLeaves          = Get-Mean -Values $RetainedValues
    MeanFseReconstructedRecords    = Get-Mean -Values $ReconstructedValues
    MeanBaselineEvaluatedRecords   = Get-Mean -Values $BaselineRecordValues
    MeanCandidateRatio             = Get-Mean -Values $CandidateValues
    MeanNodesPerRecord             = Get-Mean -Values $NodesPerRecordValues
  }
}