param(
  [string]$Label = "local",
  [string]$InputLabel = "",
  [string]$TargetWorkloadName = "cluster_boundary_range",
  [string]$OutputDir = "benchmark_artifacts",
  [string]$TrialOutputDir = ""
)

$ErrorActionPreference = "Stop"

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

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$InvariantCulture = [System.Globalization.CultureInfo]::InvariantCulture
$TreeBaselines = @("kd_tree", "r_tree")

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
          Trial                        = $TrialNumber
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
          SourceCsv                    = $WorkloadCsvPath
          Status                       = "missing target workload"
        }

        continue
      }

      $FseVisitedNodes = Get-WorkloadMetric -Row $Row -FieldName "fse_visited_nodes"
      $FseReconstructedRecords = Get-WorkloadMetric -Row $Row -FieldName "fse_reconstructed_records"

      $TrialRows += [PSCustomObject]@{
        Trial                        = $TrialNumber
        BaselineName                 = $BaselineName
        WorkloadName                 = $TargetWorkloadName
        AverageTimingRatio           = Get-WorkloadMetric -Row $Row -FieldName "average_timing_ratio"
        BaselineAverageElapsedNs     = Get-WorkloadMetric -Row $Row -FieldName "baseline_average_elapsed_ns"
        FseAverageElapsedNs          = Get-WorkloadMetric -Row $Row -FieldName "fse_average_elapsed_ns"
        FseVisitedNodes              = $FseVisitedNodes
        FseRetainedLeaves            = Get-WorkloadMetric -Row $Row -FieldName "fse_retained_leaves"
        FseReconstructedRecords      = $FseReconstructedRecords
        FseMatchedRecords            = Get-WorkloadMetric -Row $Row -FieldName "fse_matched_records"
        BaselineEvaluatedRecords     = Get-WorkloadMetric -Row $Row -FieldName "baseline_evaluated_records"
        BaselineMatchedRecords       = Get-WorkloadMetric -Row $Row -FieldName "baseline_matched_records"
        CandidateRatio               = Get-WorkloadMetric -Row $Row -FieldName "candidate_ratio"
        ReconstructionAvoidanceRatio = Get-WorkloadMetric -Row $Row -FieldName "reconstruction_avoidance_ratio"
        NodesPerRecord               = Get-RatioOrNull -Numerator $FseVisitedNodes -Denominator $FseReconstructedRecords
        SourceCsv                    = $WorkloadCsvPath
        Status                       = "loaded"
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
    BaselineName                     = $BaselineName
    WorkloadName                     = $TargetWorkloadName
    TrialCount                       = $TimingValues.Count
    MeanTimingRatio                  = $MeanTiming
    MinTimingRatio                   = $MinTiming
    MaxTimingRatio                   = $MaxTiming
    RangeTimingRatio                 = $TimingRange
    SampleStdDevTimingRatio          = Get-SampleStdDev -Values $TimingValues -Mean $MeanTiming
    MeanBaselineAverageElapsedNs     = Get-Mean -Values $BaselineAverageValues
    MeanFseAverageElapsedNs          = Get-Mean -Values $FseAverageValues
    MeanFseVisitedNodes              = Get-Mean -Values $VisitedValues
    MeanFseRetainedLeaves            = Get-Mean -Values $RetainedValues
    MeanFseReconstructedRecords      = Get-Mean -Values $ReconstructedValues
    MeanFseMatchedRecords            = Get-Mean -Values $MatchedValues
    MeanBaselineEvaluatedRecords     = Get-Mean -Values $BaselineEvaluatedValues
    MeanBaselineMatchedRecords       = Get-Mean -Values $BaselineMatchedValues
    MeanCandidateRatio               = Get-Mean -Values $CandidateValues
    MeanReconstructionAvoidanceRatio = Get-Mean -Values $AvoidanceValues
    MeanNodesPerRecord               = Get-Mean -Values $NodesPerRecordValues
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
  Add-Utf8Text -Path $NotesPath -Text "mean FSE average ns: $(Format-InvariantDouble -Value $Summary.MeanFseAverageElapsedNs)`r`n"
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
Add-Utf8Text -Path $NotesPath -Text "Use this target summary when a boundary optimization improves cluster_boundary_range enough that it may stop being the weakest workload.`r`n"
Add-Utf8Text -Path $NotesPath -Text "The weakest-workload summary answers what lost most often.`r`n"
Add-Utf8Text -Path $NotesPath -Text "This target-workload summary answers what happened to the workload we were intentionally targeting.`r`n"

Write-Host ""
Write-Host "Target workload trial artifacts written:"
Write-Host "  $DetailCsvPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $NotesPath"