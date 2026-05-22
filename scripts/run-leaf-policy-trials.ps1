param(
  [string]$Label = "local",
  [ValidateSet("small", "large")]
  [string]$Dataset = "small",
  [int]$Trials = 5,
  [int]$Iterations = 10000,
  [string]$OutputDir = "benchmark_artifacts",
  [string]$TargetWorkloadName = "",
  [double]$NoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Label)) {
  throw "`-Label` cannot be empty"
}

if ($Trials -lt 1) {
  throw "`-Trials` must be at least 1"
}

if ([string]::IsNullOrWhiteSpace($TargetWorkloadName)) {
  if ($Dataset -eq "large") {
    $TargetWorkloadName = "large_cross_cluster_boundary"
  }
  else {
    $TargetWorkloadName = "cluster_boundary_range"
  }
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TrialOutputDir = Join-Path $OutputDir "leaf-policy-trials-$Label"
New-Item -ItemType Directory -Force -Path $TrialOutputDir | Out-Null

$TrialLogPath = Join-Path $TrialOutputDir "leaf-policy-trial-output-$Label.txt"
$DetailCsvPath = Join-Path $OutputDir "leaf-policy-trial-details-$Label.csv"
$SummaryCsvPath = Join-Path $OutputDir "leaf-policy-trial-summary-$Label.csv"
$TargetDetailCsvPath = Join-Path $OutputDir "leaf-policy-target-trial-details-$Label.csv"
$TargetSummaryCsvPath = Join-Path $OutputDir "leaf-policy-target-trial-summary-$Label.csv"
$NotesPath = Join-Path $OutputDir "leaf-policy-trial-notes-$Label.txt"

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$InvariantCulture = [System.Globalization.CultureInfo]::InvariantCulture
$TreeBaselines = @("kd_tree", "r_tree")

$LeafPolicies = @(
  [PSCustomObject]@{
    PolicyName     = "8/8"
    PolicyFileName = "8-8"
    TargetLeafSize = 8
    MaxLeafSize    = 8
  },
  [PSCustomObject]@{
    PolicyName     = "4/8"
    PolicyFileName = "4-8"
    TargetLeafSize = 4
    MaxLeafSize    = 8
  }
)

function Set-Utf8Text {
  param(
    [string]$Path,
    [string]$Text
  )

  [System.IO.File]::WriteAllText($Path, $Text, $Utf8NoBom)
}

function Add-Utf8Text {
  param(
    [string]$Path,
    [string]$Text
  )

  [System.IO.File]::AppendAllText($Path, $Text, $Utf8NoBom)
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

function Convert-ToInvariantDouble {
  param(
    [string]$Value
  )

  return [double]::Parse($Value, $InvariantCulture)
}

function Format-InvariantDouble {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return "unavailable"
  }

  try {
    $DoubleValue = [System.Convert]::ToDouble($Value, $InvariantCulture)
    return $DoubleValue.ToString("0.000000", $InvariantCulture)
  }
  catch {
    return "unavailable"
  }
}

function Format-OptionalCsvNumber {
  param(
    [object]$Value
  )

  if ($null -eq $Value) {
    return ""
  }

  try {
    $DoubleValue = [System.Convert]::ToDouble($Value, $InvariantCulture)
    return $DoubleValue.ToString("0.000000", $InvariantCulture)
  }
  catch {
    return ""
  }
}

function Get-CsvMetric {
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

function Get-CsvText {
  param(
    [object]$Row,
    [string]$FieldName
  )

  if ($null -eq $Row) {
    return ""
  }

  $RawValue = $Row.$FieldName

  if ($null -eq $RawValue) {
    return ""
  }

  return $RawValue.ToString()
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
    return $null
  }

  $SumSquares = 0.0
  $MeanValue = [double]$Mean

  foreach ($Value in $Values) {
    $Delta = [double]$Value - $MeanValue
    $SumSquares += $Delta * $Delta
  }

  return [Math]::Sqrt($SumSquares / ($Values.Count - 1))
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

function Get-Classification {
  param(
    [object]$Delta,
    [double]$Threshold
  )

  if ($null -eq $Delta) {
    return "reference"
  }

  if ($Delta -ge $Threshold) {
    return "improved"
  }

  if ($Delta -le (-1.0 * $Threshold)) {
    return "regressed"
  }

  return "stable/noise"
}

function Get-LowGapRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName
  )

  return $Rows | Where-Object { $_.baseline_name -eq $BaselineName } | Select-Object -First 1
}

function Get-TargetWorkloadRow {
  param(
    [object[]]$Rows,
    [string]$BaselineName,
    [string]$WorkloadName
  )

  return $Rows | Where-Object {
    $_.baseline_name -eq $BaselineName -and $_.workload_name -eq $WorkloadName
  } | Select-Object -First 1
}

function Assert-BenchmarkValidationPassed {
  param(
    [object[]]$CommandOutput,
    [string]$PolicyName,
    [int]$TrialNumber
  )

  $SawValidationPass = $false
  $SawValidationFail = $false

  foreach ($Line in $CommandOutput) {
    $LineText = $Line.ToString().Trim()

    if ($LineText -eq "Validation: pass") {
      $SawValidationPass = $true
    }

    if ($LineText -eq "Validation: fail") {
      $SawValidationFail = $true
    }
  }

  if ($SawValidationFail) {
    throw "leaf policy trial validation failed for dataset $Dataset, policy $PolicyName, trial $TrialNumber"
  }

  if (!$SawValidationPass) {
    throw "leaf policy trial validation status was not found for dataset $Dataset, policy $PolicyName, trial $TrialNumber"
  }
}

function Invoke-LeafPolicyTrial {
  param(
    [int]$TrialNumber,
    [object]$Policy,
    [string]$LowGapCsvPath,
    [string]$WorkloadsCsvPath
  )

  $IterationsText = $Iterations.ToString()
  $TargetLeafSizeText = $Policy.TargetLeafSize.ToString()
  $MaxLeafSizeText = $Policy.MaxLeafSize.ToString()

  $CargoArguments = @(
    "run",
    "--quiet",
    "--release",
    "--",
    "--dataset",
    $Dataset,
    "--all-baselines",
    "--iterations",
    $IterationsText,
    "--target-leaf-size",
    $TargetLeafSizeText,
    "--max-leaf-size",
    $MaxLeafSizeText,
    "--csv-low-selectivity-gap",
    $LowGapCsvPath,
    "--csv-workloads",
    $WorkloadsCsvPath
  )

  Add-Utf8Text -Path $TrialLogPath -Text "`r`n---`r`n`r`n"
  Add-Utf8Text -Path $TrialLogPath -Text "Policy $($Policy.PolicyName), trial $TrialNumber`r`n"
  Add-Utf8Text -Path $TrialLogPath -Text "Low-gap CSV: $LowGapCsvPath`r`n"
  Add-Utf8Text -Path $TrialLogPath -Text "Workloads CSV: $WorkloadsCsvPath`r`n`r`n"

  $CommandOutput = & cargo @CargoArguments 2>&1
  $ExitCode = $LASTEXITCODE

  foreach ($Line in $CommandOutput) {
    $LineText = $Line.ToString()
    Write-Host $LineText
    Add-Utf8Text -Path $TrialLogPath -Text "$LineText`r`n"
  }

  if ($ExitCode -ne 0) {
    throw "leaf policy trial failed for dataset $Dataset, policy $($Policy.PolicyName), trial $TrialNumber with exit code $ExitCode"
  }

  Assert-BenchmarkValidationPassed `
    -CommandOutput $CommandOutput `
    -PolicyName $Policy.PolicyName `
    -TrialNumber $TrialNumber

  if (!(Test-Path -LiteralPath $LowGapCsvPath)) {
    throw "leaf policy trial did not write expected low-gap CSV: $LowGapCsvPath"
  }

  if (!(Test-Path -LiteralPath $WorkloadsCsvPath)) {
    throw "leaf policy trial did not write expected workloads CSV: $WorkloadsCsvPath"
  }
}

function Read-LowGapTrialRows {
  param(
    [int]$TrialNumber,
    [object]$Policy,
    [string]$LowGapCsvPath
  )

  $Rows = @(Import-Csv -LiteralPath $LowGapCsvPath)
  $TrialRows = @()

  foreach ($BaselineName in $TreeBaselines) {
    $Row = Get-LowGapRow -Rows $Rows -BaselineName $BaselineName

    if ($null -eq $Row) {
      $TrialRows += [PSCustomObject]@{
        Trial                     = $TrialNumber
        PolicyName                = $Policy.PolicyName
        TargetLeafSize            = $Policy.TargetLeafSize
        MaxLeafSize               = $Policy.MaxLeafSize
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
      PolicyName                = $Policy.PolicyName
      TargetLeafSize            = $Policy.TargetLeafSize
      MaxLeafSize               = $Policy.MaxLeafSize
      BaselineName              = $BaselineName
      LowMeanTimingRatio        = Get-CsvMetric -Row $Row -FieldName "low_mean_timing_ratio"
      LowWeightedCandidateRatio = Get-CsvMetric -Row $Row -FieldName "low_weighted_candidate_ratio"
      LowWeightedAvoidanceRatio = Get-CsvMetric -Row $Row -FieldName "low_weighted_reconstruction_avoidance_ratio"
      SourceCsv                 = $LowGapCsvPath
      Status                    = "loaded"
    }
  }

  return $TrialRows
}

function Read-TargetWorkloadTrialRows {
  param(
    [int]$TrialNumber,
    [object]$Policy,
    [string]$WorkloadsCsvPath
  )

  $Rows = @(Import-Csv -LiteralPath $WorkloadsCsvPath)
  $TrialRows = @()

  foreach ($BaselineName in $TreeBaselines) {
    $Row = Get-TargetWorkloadRow `
      -Rows $Rows `
      -BaselineName $BaselineName `
      -WorkloadName $TargetWorkloadName

    if ($null -eq $Row) {
      $TrialRows += [PSCustomObject]@{
        Trial                        = $TrialNumber
        PolicyName                   = $Policy.PolicyName
        TargetLeafSize               = $Policy.TargetLeafSize
        MaxLeafSize                  = $Policy.MaxLeafSize
        BaselineName                 = $BaselineName
        WorkloadName                 = $TargetWorkloadName
        AverageTimingRatio           = $null
        BaselineAverageElapsedNs     = $null
        FseAverageElapsedNs          = $null
        FseVisitedNodes              = $null
        FseRetainedLeaves            = $null
        FseReconstructedRecords      = $null
        FseMatchedRecords            = $null
        BaselineEvaluatedRecords     = $null
        BaselineMatchedRecords       = $null
        CandidateRatio               = $null
        ReconstructionAvoidanceRatio = $null
        NodesPerRecord               = $null
        SourceCsv                    = $WorkloadsCsvPath
        Status                       = "missing target workload"
      }

      continue
    }

    $FseVisitedNodes = Get-CsvMetric -Row $Row -FieldName "fse_visited_nodes"
    $FseReconstructedRecords = Get-CsvMetric -Row $Row -FieldName "fse_reconstructed_records"

    $TrialRows += [PSCustomObject]@{
      Trial                        = $TrialNumber
      PolicyName                   = $Policy.PolicyName
      TargetLeafSize               = $Policy.TargetLeafSize
      MaxLeafSize                  = $Policy.MaxLeafSize
      BaselineName                 = $BaselineName
      WorkloadName                 = Get-CsvText -Row $Row -FieldName "workload_name"
      AverageTimingRatio           = Get-CsvMetric -Row $Row -FieldName "average_timing_ratio"
      BaselineAverageElapsedNs     = Get-CsvMetric -Row $Row -FieldName "baseline_average_elapsed_ns"
      FseAverageElapsedNs          = Get-CsvMetric -Row $Row -FieldName "fse_average_elapsed_ns"
      FseVisitedNodes              = $FseVisitedNodes
      FseRetainedLeaves            = Get-CsvMetric -Row $Row -FieldName "fse_retained_leaves"
      FseReconstructedRecords      = $FseReconstructedRecords
      FseMatchedRecords            = Get-CsvMetric -Row $Row -FieldName "fse_matched_records"
      BaselineEvaluatedRecords     = Get-CsvMetric -Row $Row -FieldName "baseline_evaluated_records"
      BaselineMatchedRecords       = Get-CsvMetric -Row $Row -FieldName "baseline_matched_records"
      CandidateRatio               = Get-CsvMetric -Row $Row -FieldName "candidate_ratio"
      ReconstructionAvoidanceRatio = Get-CsvMetric -Row $Row -FieldName "reconstruction_avoidance_ratio"
      NodesPerRecord               = Get-RatioOrNull -Numerator $FseVisitedNodes -Denominator $FseReconstructedRecords
      SourceCsv                    = $WorkloadsCsvPath
      Status                       = "loaded"
    }
  }

  return $TrialRows
}

function New-LowGapSummaryRow {
  param(
    [object]$Policy,
    [string]$BaselineName,
    [object[]]$Rows,
    [object]$ReferenceRow
  )

  $PolicyRows = @($Rows | Where-Object {
      $_.PolicyName -eq $Policy.PolicyName `
        -and $_.BaselineName -eq $BaselineName `
        -and $null -ne $_.LowMeanTimingRatio
    })

  $TimingValues = @($PolicyRows | ForEach-Object { [double]$_.LowMeanTimingRatio })
  $CandidateValues = @($PolicyRows | Where-Object { $null -ne $_.LowWeightedCandidateRatio } | ForEach-Object { [double]$_.LowWeightedCandidateRatio })
  $AvoidanceValues = @($PolicyRows | Where-Object { $null -ne $_.LowWeightedAvoidanceRatio } | ForEach-Object { [double]$_.LowWeightedAvoidanceRatio })

  $MeanTiming = Get-Mean -Values $TimingValues
  $MinTiming = Get-Minimum -Values $TimingValues
  $MaxTiming = Get-Maximum -Values $TimingValues
  $RangeTiming = $null

  if ($null -ne $MinTiming -and $null -ne $MaxTiming) {
    $RangeTiming = $MaxTiming - $MinTiming
  }

  $StdDevTiming = Get-SampleStdDev -Values $TimingValues -Mean $MeanTiming
  $MeanCandidate = Get-Mean -Values $CandidateValues
  $MeanAvoidance = Get-Mean -Values $AvoidanceValues

  $ReferenceMeanTiming = $null
  $TimingDelta = $null
  $CandidateDelta = $null
  $AvoidanceDelta = $null

  if ($null -ne $ReferenceRow) {
    $ReferenceMeanTiming = $ReferenceRow.MeanLowTimingRatio

    if ($null -ne $MeanTiming -and $null -ne $ReferenceMeanTiming) {
      $TimingDelta = $MeanTiming - $ReferenceMeanTiming
    }

    if ($null -ne $MeanCandidate -and $null -ne $ReferenceRow.MeanLowWeightedCandidateRatio) {
      $CandidateDelta = $MeanCandidate - $ReferenceRow.MeanLowWeightedCandidateRatio
    }

    if ($null -ne $MeanAvoidance -and $null -ne $ReferenceRow.MeanLowWeightedAvoidanceRatio) {
      $AvoidanceDelta = $MeanAvoidance - $ReferenceRow.MeanLowWeightedAvoidanceRatio
    }
  }

  return [PSCustomObject]@{
    PolicyName                         = $Policy.PolicyName
    TargetLeafSize                     = $Policy.TargetLeafSize
    MaxLeafSize                        = $Policy.MaxLeafSize
    BaselineName                       = $BaselineName
    TrialCount                         = $TimingValues.Count
    MeanLowTimingRatio                 = $MeanTiming
    MinLowTimingRatio                  = $MinTiming
    MaxLowTimingRatio                  = $MaxTiming
    RangeLowTimingRatio                = $RangeTiming
    SampleStdDevLowTimingRatio         = $StdDevTiming
    ReferencePolicyName                = $(if ($null -eq $ReferenceRow) { "" } else { $ReferenceRow.PolicyName })
    ReferenceMeanLowTimingRatio        = $ReferenceMeanTiming
    LowTimingDeltaVsReference          = $TimingDelta
    LowTimingClassificationVsReference = Get-Classification -Delta $TimingDelta -Threshold $NoiseThreshold
    MeanLowWeightedCandidateRatio      = $MeanCandidate
    MeanLowWeightedAvoidanceRatio      = $MeanAvoidance
    CandidateDeltaVsReference          = $CandidateDelta
    AvoidanceDeltaVsReference          = $AvoidanceDelta
  }
}

function New-TargetSummaryRow {
  param(
    [object]$Policy,
    [string]$BaselineName,
    [object[]]$Rows,
    [object]$ReferenceRow
  )

  $PolicyRows = @($Rows | Where-Object {
      $_.PolicyName -eq $Policy.PolicyName `
        -and $_.BaselineName -eq $BaselineName `
        -and $null -ne $_.AverageTimingRatio
    })

  $TimingValues = @($PolicyRows | ForEach-Object { [double]$_.AverageTimingRatio })
  $BaselineAverageValues = @($PolicyRows | Where-Object { $null -ne $_.BaselineAverageElapsedNs } | ForEach-Object { [double]$_.BaselineAverageElapsedNs })
  $FseAverageValues = @($PolicyRows | Where-Object { $null -ne $_.FseAverageElapsedNs } | ForEach-Object { [double]$_.FseAverageElapsedNs })
  $VisitedNodeValues = @($PolicyRows | Where-Object { $null -ne $_.FseVisitedNodes } | ForEach-Object { [double]$_.FseVisitedNodes })
  $RetainedLeafValues = @($PolicyRows | Where-Object { $null -ne $_.FseRetainedLeaves } | ForEach-Object { [double]$_.FseRetainedLeaves })
  $ReconstructedRecordValues = @($PolicyRows | Where-Object { $null -ne $_.FseReconstructedRecords } | ForEach-Object { [double]$_.FseReconstructedRecords })
  $MatchedRecordValues = @($PolicyRows | Where-Object { $null -ne $_.FseMatchedRecords } | ForEach-Object { [double]$_.FseMatchedRecords })
  $BaselineEvaluatedRecordValues = @($PolicyRows | Where-Object { $null -ne $_.BaselineEvaluatedRecords } | ForEach-Object { [double]$_.BaselineEvaluatedRecords })
  $BaselineMatchedRecordValues = @($PolicyRows | Where-Object { $null -ne $_.BaselineMatchedRecords } | ForEach-Object { [double]$_.BaselineMatchedRecords })
  $CandidateValues = @($PolicyRows | Where-Object { $null -ne $_.CandidateRatio } | ForEach-Object { [double]$_.CandidateRatio })
  $AvoidanceValues = @($PolicyRows | Where-Object { $null -ne $_.ReconstructionAvoidanceRatio } | ForEach-Object { [double]$_.ReconstructionAvoidanceRatio })
  $NodesPerRecordValues = @($PolicyRows | Where-Object { $null -ne $_.NodesPerRecord } | ForEach-Object { [double]$_.NodesPerRecord })

  $MeanTiming = Get-Mean -Values $TimingValues
  $MinTiming = Get-Minimum -Values $TimingValues
  $MaxTiming = Get-Maximum -Values $TimingValues
  $RangeTiming = $null

  if ($null -ne $MinTiming -and $null -ne $MaxTiming) {
    $RangeTiming = $MaxTiming - $MinTiming
  }

  $StdDevTiming = Get-SampleStdDev -Values $TimingValues -Mean $MeanTiming
  $MeanBaselineAverage = Get-Mean -Values $BaselineAverageValues
  $MeanFseAverage = Get-Mean -Values $FseAverageValues
  $MeanVisitedNodes = Get-Mean -Values $VisitedNodeValues
  $MeanRetainedLeaves = Get-Mean -Values $RetainedLeafValues
  $MeanReconstructedRecords = Get-Mean -Values $ReconstructedRecordValues
  $MeanMatchedRecords = Get-Mean -Values $MatchedRecordValues
  $MeanBaselineEvaluatedRecords = Get-Mean -Values $BaselineEvaluatedRecordValues
  $MeanBaselineMatchedRecords = Get-Mean -Values $BaselineMatchedRecordValues
  $MeanCandidate = Get-Mean -Values $CandidateValues
  $MeanAvoidance = Get-Mean -Values $AvoidanceValues
  $MeanNodesPerRecord = Get-Mean -Values $NodesPerRecordValues

  $ReferenceMeanTiming = $null
  $TimingDelta = $null
  $VisitedNodesDelta = $null
  $ReconstructedRecordsDelta = $null
  $CandidateDelta = $null
  $MatchedRecordsDelta = $null
  $BaselineMatchedRecordsDelta = $null

  if ($null -ne $ReferenceRow) {
    $ReferenceMeanTiming = $ReferenceRow.MeanTimingRatio

    if ($null -ne $MeanTiming -and $null -ne $ReferenceMeanTiming) {
      $TimingDelta = $MeanTiming - $ReferenceMeanTiming
    }

    if ($null -ne $MeanVisitedNodes -and $null -ne $ReferenceRow.MeanFseVisitedNodes) {
      $VisitedNodesDelta = $MeanVisitedNodes - $ReferenceRow.MeanFseVisitedNodes
    }

    if ($null -ne $MeanReconstructedRecords -and $null -ne $ReferenceRow.MeanFseReconstructedRecords) {
      $ReconstructedRecordsDelta = $MeanReconstructedRecords - $ReferenceRow.MeanFseReconstructedRecords
    }

    if ($null -ne $MeanCandidate -and $null -ne $ReferenceRow.MeanCandidateRatio) {
      $CandidateDelta = $MeanCandidate - $ReferenceRow.MeanCandidateRatio
    }

    if ($null -ne $MeanMatchedRecords -and $null -ne $ReferenceRow.MeanFseMatchedRecords) {
      $MatchedRecordsDelta = $MeanMatchedRecords - $ReferenceRow.MeanFseMatchedRecords
    }

    if ($null -ne $MeanBaselineMatchedRecords -and $null -ne $ReferenceRow.MeanBaselineMatchedRecords) {
      $BaselineMatchedRecordsDelta = $MeanBaselineMatchedRecords - $ReferenceRow.MeanBaselineMatchedRecords
    }
  }

  return [PSCustomObject]@{
    PolicyName                          = $Policy.PolicyName
    TargetLeafSize                      = $Policy.TargetLeafSize
    MaxLeafSize                         = $Policy.MaxLeafSize
    BaselineName                        = $BaselineName
    WorkloadName                        = $TargetWorkloadName
    TrialCount                          = $TimingValues.Count
    MeanTimingRatio                     = $MeanTiming
    MinTimingRatio                      = $MinTiming
    MaxTimingRatio                      = $MaxTiming
    RangeTimingRatio                    = $RangeTiming
    SampleStdDevTimingRatio             = $StdDevTiming
    ReferencePolicyName                 = $(if ($null -eq $ReferenceRow) { "" } else { $ReferenceRow.PolicyName })
    ReferenceMeanTimingRatio            = $ReferenceMeanTiming
    TimingDeltaVsReference              = $TimingDelta
    TimingClassificationVsReference     = Get-Classification -Delta $TimingDelta -Threshold $NoiseThreshold
    MeanBaselineAverageElapsedNs        = $MeanBaselineAverage
    MeanFseAverageElapsedNs             = $MeanFseAverage
    MeanFseVisitedNodes                 = $MeanVisitedNodes
    MeanFseRetainedLeaves               = $MeanRetainedLeaves
    MeanFseReconstructedRecords         = $MeanReconstructedRecords
    MeanFseMatchedRecords               = $MeanMatchedRecords
    MeanBaselineEvaluatedRecords        = $MeanBaselineEvaluatedRecords
    MeanBaselineMatchedRecords          = $MeanBaselineMatchedRecords
    MeanCandidateRatio                  = $MeanCandidate
    MeanReconstructionAvoidanceRatio    = $MeanAvoidance
    MeanNodesPerRecord                  = $MeanNodesPerRecord
    MeanFseVisitedNodesDeltaVsReference = $VisitedNodesDelta
    MeanReconstructedDeltaVsReference   = $ReconstructedRecordsDelta
    MeanCandidateDeltaVsReference       = $CandidateDelta
    MeanFseMatchedDeltaVsReference      = $MatchedRecordsDelta
    MeanBaselineMatchedDeltaVsReference = $BaselineMatchedRecordsDelta
  }
}

function Get-BaselineClassificationText {
  param(
    [object[]]$Rows,
    [string]$PolicyName,
    [string]$ClassificationFieldName
  )

  $Parts = @()

  foreach ($BaselineName in $TreeBaselines) {
    $Row = $Rows | Where-Object {
      $_.PolicyName -eq $PolicyName -and $_.BaselineName -eq $BaselineName
    } | Select-Object -First 1

    if ($null -eq $Row) {
      $Parts += "$BaselineName=missing"
      continue
    }

    $Parts += "$BaselineName=$($Row.$ClassificationFieldName)"
  }

  return ($Parts -join ", ")
}

function Get-PolicyRows {
  param(
    [object[]]$Rows,
    [string]$PolicyName
  )

  return @($Rows | Where-Object { $_.PolicyName -eq $PolicyName })
}

function Test-AnyPolicyRegression {
  param(
    [object[]]$Rows,
    [string]$PolicyName,
    [string]$ClassificationFieldName
  )

  $PolicyRows = Get-PolicyRows -Rows $Rows -PolicyName $PolicyName

  foreach ($Row in $PolicyRows) {
    if ($Row.$ClassificationFieldName -eq "regressed") {
      return $true
    }
  }

  return $false
}

function Test-AllPolicyImproved {
  param(
    [object[]]$Rows,
    [string]$PolicyName,
    [string]$ClassificationFieldName
  )

  $PolicyRows = Get-PolicyRows -Rows $Rows -PolicyName $PolicyName

  if ($PolicyRows.Count -eq 0) {
    return $false
  }

  foreach ($Row in $PolicyRows) {
    if ($Row.$ClassificationFieldName -ne "improved") {
      return $false
    }
  }

  return $true
}

function Write-PolicyVerdictNotes {
  param(
    [object[]]$LowGapSummaryRows,
    [object[]]$TargetSummaryRows
  )

  Add-Utf8Text -Path $NotesPath -Text "`r`nPolicy verdict`r`n"
  Add-Utf8Text -Path $NotesPath -Text "--------------`r`n"

  foreach ($Policy in $LeafPolicies) {
    if ($Policy.PolicyName -eq "8/8") {
      continue
    }

    $LowClassifications = Get-BaselineClassificationText `
      -Rows $LowGapSummaryRows `
      -PolicyName $Policy.PolicyName `
      -ClassificationFieldName "LowTimingClassificationVsReference"

    $TargetClassifications = Get-BaselineClassificationText `
      -Rows $TargetSummaryRows `
      -PolicyName $Policy.PolicyName `
      -ClassificationFieldName "TimingClassificationVsReference"

    $HasLowRegression = Test-AnyPolicyRegression `
      -Rows $LowGapSummaryRows `
      -PolicyName $Policy.PolicyName `
      -ClassificationFieldName "LowTimingClassificationVsReference"

    $HasTargetRegression = Test-AnyPolicyRegression `
      -Rows $TargetSummaryRows `
      -PolicyName $Policy.PolicyName `
      -ClassificationFieldName "TimingClassificationVsReference"

    $AllLowImproved = Test-AllPolicyImproved `
      -Rows $LowGapSummaryRows `
      -PolicyName $Policy.PolicyName `
      -ClassificationFieldName "LowTimingClassificationVsReference"

    $AllTargetImproved = Test-AllPolicyImproved `
      -Rows $TargetSummaryRows `
      -PolicyName $Policy.PolicyName `
      -ClassificationFieldName "TimingClassificationVsReference"

    Add-Utf8Text -Path $NotesPath -Text "`r`ncandidate policy: $($Policy.PolicyName)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "low-selectivity timing classifications: $LowClassifications`r`n"
    Add-Utf8Text -Path $NotesPath -Text "target timing classifications: $TargetClassifications`r`n"

    if ($HasLowRegression -or $HasTargetRegression) {
      Add-Utf8Text -Path $NotesPath -Text "verdict: keep 8/8; $($Policy.PolicyName) regresses timing beyond the noise threshold despite any candidate reduction.`r`n"
      continue
    }

    if ($AllLowImproved -and $AllTargetImproved) {
      Add-Utf8Text -Path $NotesPath -Text "verdict: $($Policy.PolicyName) is a promotion candidate; low-selectivity and target timing both improved beyond the noise threshold.`r`n"
      continue
    }

    Add-Utf8Text -Path $NotesPath -Text "verdict: keep 8/8 pending manual review; $($Policy.PolicyName) did not clearly improve both low-selectivity and target timing.`r`n"
  }
}

function Write-Notes {
  param(
    [object[]]$LowGapSummaryRows,
    [object[]]$TargetSummaryRows
  )

  Set-Utf8Text -Path $NotesPath -Text "Leaf policy trial notes`r`n"
  Add-Utf8Text -Path $NotesPath -Text "=======================`r`n`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Label: $Label`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Trials per policy: $Trials`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Iterations per trial: $Iterations`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Target workload: $TargetWorkloadName`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Timing ratio meaning: baseline elapsed / FSE elapsed; higher is better for FSE.`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Reference policy: 8/8`r`n`r`n"

  Add-Utf8Text -Path $NotesPath -Text "Low-selectivity policy summary`r`n"
  Add-Utf8Text -Path $NotesPath -Text "------------------------------`r`n"

  foreach ($Row in $LowGapSummaryRows) {
    Add-Utf8Text -Path $NotesPath -Text "`r`n$($Row.BaselineName) $($Row.PolicyName)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "trials loaded: $($Row.TrialCount)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean low timing: $(Format-InvariantDouble -Value $Row.MeanLowTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean weighted candidate: $(Format-InvariantDouble -Value $Row.MeanLowWeightedCandidateRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean weighted avoidance: $(Format-InvariantDouble -Value $Row.MeanLowWeightedAvoidanceRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "delta vs reference: $(Format-InvariantDouble -Value $Row.LowTimingDeltaVsReference)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "classification vs reference: $($Row.LowTimingClassificationVsReference)`r`n"
  }

  Add-Utf8Text -Path $NotesPath -Text "`r`nTarget workload policy summary`r`n"
  Add-Utf8Text -Path $NotesPath -Text "------------------------------`r`n"

  foreach ($Row in $TargetSummaryRows) {
    Add-Utf8Text -Path $NotesPath -Text "`r`n$($Row.BaselineName) $($Row.PolicyName)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "trials loaded: $($Row.TrialCount)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean target timing: $(Format-InvariantDouble -Value $Row.MeanTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE visited nodes: $(Format-InvariantDouble -Value $Row.MeanFseVisitedNodes)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE retained leaves: $(Format-InvariantDouble -Value $Row.MeanFseRetainedLeaves)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE reconstructed records: $(Format-InvariantDouble -Value $Row.MeanFseReconstructedRecords)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE matched records: $(Format-InvariantDouble -Value $Row.MeanFseMatchedRecords)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean candidate ratio: $(Format-InvariantDouble -Value $Row.MeanCandidateRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "delta vs reference: $(Format-InvariantDouble -Value $Row.TimingDeltaVsReference)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "classification vs reference: $($Row.TimingClassificationVsReference)`r`n"
  }

  Write-PolicyVerdictNotes `
    -LowGapSummaryRows $LowGapSummaryRows `
    -TargetSummaryRows $TargetSummaryRows

  Add-Utf8Text -Path $NotesPath -Text "`r`nDecision guidance`r`n"
  Add-Utf8Text -Path $NotesPath -Text "-----------------`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Use this script before changing leaf policy defaults or adding policy-selection logic.`r`n"
  Add-Utf8Text -Path $NotesPath -Text "A tighter 4/8 policy is useful only if repeated-trial timing improves beyond the threshold or the candidate reduction is worth an intentional timing tradeoff.`r`n"
  Add-Utf8Text -Path $NotesPath -Text "For boundary-focused work, prioritize the target workload rows for $TargetWorkloadName.`r`n"
  Add-Utf8Text -Path $NotesPath -Text "Do not accept a leaf-policy change based on one normal benchmark run.`r`n"
}

Set-Utf8Text -Path $TrialLogPath -Text "Leaf policy trial output`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "========================`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Target workload: $TargetWorkloadName`r`n"

$LowGapTrialRows = @()
$TargetTrialRows = @()

foreach ($TrialNumber in 1..$Trials) {
  foreach ($Policy in $LeafPolicies) {
    $TrialText = $TrialNumber.ToString("000", $InvariantCulture)
    $LowGapCsvPath = Join-Path $TrialOutputDir "low-selectivity-gap-$Label-policy-$($Policy.PolicyFileName)-trial-$TrialText.csv"
    $WorkloadsCsvPath = Join-Path $TrialOutputDir "workloads-$Label-policy-$($Policy.PolicyFileName)-trial-$TrialText.csv"

    Write-Host ""
    Write-Host "Running leaf policy trial $TrialNumber of $Trials"
    Write-Host "  policy: $($Policy.PolicyName)"
    Write-Host "  $LowGapCsvPath"
    Write-Host "  $WorkloadsCsvPath"

    Invoke-LeafPolicyTrial `
      -TrialNumber $TrialNumber `
      -Policy $Policy `
      -LowGapCsvPath $LowGapCsvPath `
      -WorkloadsCsvPath $WorkloadsCsvPath

    $LowGapTrialRows += Read-LowGapTrialRows `
      -TrialNumber $TrialNumber `
      -Policy $Policy `
      -LowGapCsvPath $LowGapCsvPath

    $TargetTrialRows += Read-TargetWorkloadTrialRows `
      -TrialNumber $TrialNumber `
      -Policy $Policy `
      -WorkloadsCsvPath $WorkloadsCsvPath
  }
}

$LowGapSummaryRows = @()
$TargetSummaryRows = @()

foreach ($Policy in $LeafPolicies) {
  foreach ($BaselineName in $TreeBaselines) {
    $ReferenceLowGapRow = $null
    $ReferenceTargetRow = $null

    if ($Policy.PolicyName -ne "8/8") {
      $ReferenceLowGapRow = $LowGapSummaryRows | Where-Object {
        $_.PolicyName -eq "8/8" -and $_.BaselineName -eq $BaselineName
      } | Select-Object -First 1

      $ReferenceTargetRow = $TargetSummaryRows | Where-Object {
        $_.PolicyName -eq "8/8" -and $_.BaselineName -eq $BaselineName
      } | Select-Object -First 1
    }

    $LowGapSummaryRows += New-LowGapSummaryRow `
      -Policy $Policy `
      -BaselineName $BaselineName `
      -Rows $LowGapTrialRows `
      -ReferenceRow $ReferenceLowGapRow

    $TargetSummaryRows += New-TargetSummaryRow `
      -Policy $Policy `
      -BaselineName $BaselineName `
      -Rows $TargetTrialRows `
      -ReferenceRow $ReferenceTargetRow
  }
}

$DetailHeader = @(
  "trial",
  "policy_name",
  "target_leaf_size",
  "max_leaf_size",
  "baseline_name",
  "low_mean_timing_ratio",
  "low_weighted_candidate_ratio",
  "low_weighted_avoidance_ratio",
  "source_csv",
  "status"
)

$DetailRows = @()

foreach ($Row in $LowGapTrialRows) {
  $DetailRows += , @(
    $Row.Trial,
    $Row.PolicyName,
    $Row.TargetLeafSize,
    $Row.MaxLeafSize,
    $Row.BaselineName,
    $(Format-OptionalCsvNumber -Value $Row.LowMeanTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.LowWeightedCandidateRatio),
    $(Format-OptionalCsvNumber -Value $Row.LowWeightedAvoidanceRatio),
    $Row.SourceCsv,
    $Row.Status
  )
}

Write-CsvDocument `
  -Path $DetailCsvPath `
  -Header $DetailHeader `
  -Rows $DetailRows

$SummaryHeader = @(
  "policy_name",
  "target_leaf_size",
  "max_leaf_size",
  "baseline_name",
  "trial_count",
  "mean_low_timing_ratio",
  "min_low_timing_ratio",
  "max_low_timing_ratio",
  "range_low_timing_ratio",
  "sample_stddev_low_timing_ratio",
  "reference_policy_name",
  "reference_mean_low_timing_ratio",
  "low_timing_delta_vs_reference",
  "low_timing_classification_vs_reference",
  "mean_low_weighted_candidate_ratio",
  "mean_low_weighted_avoidance_ratio",
  "candidate_delta_vs_reference",
  "avoidance_delta_vs_reference"
)

$SummaryRows = @()

foreach ($Row in $LowGapSummaryRows) {
  $SummaryRows += , @(
    $Row.PolicyName,
    $Row.TargetLeafSize,
    $Row.MaxLeafSize,
    $Row.BaselineName,
    $Row.TrialCount,
    $(Format-OptionalCsvNumber -Value $Row.MeanLowTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.MinLowTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.MaxLowTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.RangeLowTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.SampleStdDevLowTimingRatio),
    $Row.ReferencePolicyName,
    $(Format-OptionalCsvNumber -Value $Row.ReferenceMeanLowTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.LowTimingDeltaVsReference),
    $Row.LowTimingClassificationVsReference,
    $(Format-OptionalCsvNumber -Value $Row.MeanLowWeightedCandidateRatio),
    $(Format-OptionalCsvNumber -Value $Row.MeanLowWeightedAvoidanceRatio),
    $(Format-OptionalCsvNumber -Value $Row.CandidateDeltaVsReference),
    $(Format-OptionalCsvNumber -Value $Row.AvoidanceDeltaVsReference)
  )
}

Write-CsvDocument `
  -Path $SummaryCsvPath `
  -Header $SummaryHeader `
  -Rows $SummaryRows

$TargetDetailHeader = @(
  "trial",
  "policy_name",
  "target_leaf_size",
  "max_leaf_size",
  "baseline_name",
  "workload_name",
  "average_timing_ratio",
  "baseline_average_elapsed_ns",
  "fse_average_elapsed_ns",
  "fse_visited_nodes",
  "fse_retained_leaves",
  "fse_reconstructed_records",
  "fse_matched_records",
  "baseline_evaluated_records",
  "baseline_matched_records",
  "candidate_ratio",
  "reconstruction_avoidance_ratio",
  "nodes_per_record",
  "source_csv",
  "status"
)

$TargetDetailRows = @()

foreach ($Row in $TargetTrialRows) {
  $TargetDetailRows += , @(
    $Row.Trial,
    $Row.PolicyName,
    $Row.TargetLeafSize,
    $Row.MaxLeafSize,
    $Row.BaselineName,
    $Row.WorkloadName,
    $(Format-OptionalCsvNumber -Value $Row.AverageTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.BaselineAverageElapsedNs),
    $(Format-OptionalCsvNumber -Value $Row.FseAverageElapsedNs),
    $(Format-OptionalCsvNumber -Value $Row.FseVisitedNodes),
    $(Format-OptionalCsvNumber -Value $Row.FseRetainedLeaves),
    $(Format-OptionalCsvNumber -Value $Row.FseReconstructedRecords),
    $(Format-OptionalCsvNumber -Value $Row.FseMatchedRecords),
    $(Format-OptionalCsvNumber -Value $Row.BaselineEvaluatedRecords),
    $(Format-OptionalCsvNumber -Value $Row.BaselineMatchedRecords),
    $(Format-OptionalCsvNumber -Value $Row.CandidateRatio),
    $(Format-OptionalCsvNumber -Value $Row.ReconstructionAvoidanceRatio),
    $(Format-OptionalCsvNumber -Value $Row.NodesPerRecord),
    $Row.SourceCsv,
    $Row.Status
  )
}

Write-CsvDocument `
  -Path $TargetDetailCsvPath `
  -Header $TargetDetailHeader `
  -Rows $TargetDetailRows

$TargetSummaryHeader = @(
  "policy_name",
  "target_leaf_size",
  "max_leaf_size",
  "baseline_name",
  "workload_name",
  "trial_count",
  "mean_timing_ratio",
  "min_timing_ratio",
  "max_timing_ratio",
  "range_timing_ratio",
  "sample_stddev_timing_ratio",
  "reference_policy_name",
  "reference_mean_timing_ratio",
  "timing_delta_vs_reference",
  "timing_classification_vs_reference",
  "mean_baseline_average_elapsed_ns",
  "mean_fse_average_elapsed_ns",
  "mean_fse_visited_nodes",
  "mean_fse_retained_leaves",
  "mean_fse_reconstructed_records",
  "mean_fse_matched_records",
  "mean_baseline_evaluated_records",
  "mean_baseline_matched_records",
  "mean_candidate_ratio",
  "mean_reconstruction_avoidance_ratio",
  "mean_nodes_per_record",
  "mean_fse_visited_nodes_delta_vs_reference",
  "mean_reconstructed_delta_vs_reference",
  "mean_candidate_delta_vs_reference",
  "mean_fse_matched_delta_vs_reference",
  "mean_baseline_matched_delta_vs_reference"
)

$TargetSummaryRowsForCsv = @()

foreach ($Row in $TargetSummaryRows) {
  $TargetSummaryRowsForCsv += , @(
    $Row.PolicyName,
    $Row.TargetLeafSize,
    $Row.MaxLeafSize,
    $Row.BaselineName,
    $Row.WorkloadName,
    $Row.TrialCount,
    $(Format-OptionalCsvNumber -Value $Row.MeanTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.MinTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.MaxTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.RangeTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.SampleStdDevTimingRatio),
    $Row.ReferencePolicyName,
    $(Format-OptionalCsvNumber -Value $Row.ReferenceMeanTimingRatio),
    $(Format-OptionalCsvNumber -Value $Row.TimingDeltaVsReference),
    $Row.TimingClassificationVsReference,
    $(Format-OptionalCsvNumber -Value $Row.MeanBaselineAverageElapsedNs),
    $(Format-OptionalCsvNumber -Value $Row.MeanFseAverageElapsedNs),
    $(Format-OptionalCsvNumber -Value $Row.MeanFseVisitedNodes),
    $(Format-OptionalCsvNumber -Value $Row.MeanFseRetainedLeaves),
    $(Format-OptionalCsvNumber -Value $Row.MeanFseReconstructedRecords),
    $(Format-OptionalCsvNumber -Value $Row.MeanFseMatchedRecords),
    $(Format-OptionalCsvNumber -Value $Row.MeanBaselineEvaluatedRecords),
    $(Format-OptionalCsvNumber -Value $Row.MeanBaselineMatchedRecords),
    $(Format-OptionalCsvNumber -Value $Row.MeanCandidateRatio),
    $(Format-OptionalCsvNumber -Value $Row.MeanReconstructionAvoidanceRatio),
    $(Format-OptionalCsvNumber -Value $Row.MeanNodesPerRecord),
    $(Format-OptionalCsvNumber -Value $Row.MeanFseVisitedNodesDeltaVsReference),
    $(Format-OptionalCsvNumber -Value $Row.MeanReconstructedDeltaVsReference),
    $(Format-OptionalCsvNumber -Value $Row.MeanCandidateDeltaVsReference),
    $(Format-OptionalCsvNumber -Value $Row.MeanFseMatchedDeltaVsReference),
    $(Format-OptionalCsvNumber -Value $Row.MeanBaselineMatchedDeltaVsReference)
  )
}

Write-CsvDocument `
  -Path $TargetSummaryCsvPath `
  -Header $TargetSummaryHeader `
  -Rows $TargetSummaryRowsForCsv

Write-Notes `
  -LowGapSummaryRows $LowGapSummaryRows `
  -TargetSummaryRows $TargetSummaryRows

Write-Host ""
Write-Host "Leaf policy trial artifacts written:"
Write-Host "  $TrialOutputDir"
Write-Host "  $TrialLogPath"
Write-Host "  $DetailCsvPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $TargetDetailCsvPath"
Write-Host "  $TargetSummaryCsvPath"
Write-Host "  $NotesPath"