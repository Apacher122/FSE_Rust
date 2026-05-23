param(
    [string]$Label = "local",
    [ValidateSet("small", "large")]
    [string]$Dataset = "small",
    [ValidateRange(0, 2147483647)]
    [int]$MaxDepth = 0,
    [int]$Trials = 5,
    [int]$Iterations = 10000,
    [string]$OutputDir = "benchmark_artifacts",
    [string]$PreviousLowSelectivityGapCsv = "",
    [double]$LowGapNoiseThreshold = 0.03
)

$ErrorActionPreference = "Stop"

if ($MaxDepth -gt 0) {
    $EffectiveMaxDepth = $MaxDepth
}
elseif ($Dataset -eq "large") {
    $EffectiveMaxDepth = 16
}
else {
    $EffectiveMaxDepth = 8
}

$EffectiveMaxDepthText = $EffectiveMaxDepth.ToString()

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TrialOutputDir = Join-Path $OutputDir "low-gap-trials-$Label"
New-Item -ItemType Directory -Force -Path $TrialOutputDir | Out-Null

$TrialLogPath = Join-Path $TrialOutputDir "low-gap-trial-output-$Label.txt"
$DetailCsvPath = Join-Path $OutputDir "low-gap-trial-details-$Label.csv"
$SummaryCsvPath = Join-Path $OutputDir "low-gap-trial-summary-$Label.csv"
$WorkloadDetailCsvPath = Join-Path $OutputDir "low-gap-workload-trial-details-$Label.csv"
$WorkloadSummaryCsvPath = Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv"
$NotesPath = Join-Path $OutputDir "low-gap-trial-notes-$Label.txt"

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

function Get-LowGapRow {
    param(
        [object[]]$Rows,
        [string]$BaselineName
    )

    return $Rows | Where-Object { $_.baseline_name -eq $BaselineName } | Select-Object -First 1
}

function Get-LowGapMetric {
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

function Get-WorkloadText {
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

function Get-LowGapClassification {
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

function Invoke-LowGapTrial {
    param(
        [int]$TrialNumber,
        [string]$LowGapCsvPath,
        [string]$WorkloadsCsvPath
    )

    $CargoArguments = @(
        "run",
        "--quiet",
        "--release",
        "--",
        "--dataset",
        $Dataset,
        "--all-baselines",
        "--iterations",
        $Iterations.ToString(),
        "--max-depth",
        $EffectiveMaxDepthText,
        "--csv-low-selectivity-gap",
        $LowGapCsvPath,
        "--csv-workloads",
        $WorkloadsCsvPath
    )

    Add-Utf8Text -Path $TrialLogPath -Text "`r`n---`r`n`r`nTrial $TrialNumber`r`n"
    Add-Utf8Text -Path $TrialLogPath -Text "Dataset: $Dataset`r`n"
    Add-Utf8Text -Path $TrialLogPath -Text "Max depth: $EffectiveMaxDepth`r`n"
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
        throw "low-gap trial $TrialNumber failed with exit code $ExitCode"
    }

    if (!(Test-Path -LiteralPath $LowGapCsvPath)) {
        throw "low-gap trial $TrialNumber did not write expected CSV: $LowGapCsvPath"
    }

    if (!(Test-Path -LiteralPath $WorkloadsCsvPath)) {
        throw "low-gap trial $TrialNumber did not write expected workload CSV: $WorkloadsCsvPath"
    }
}

function Read-TrialRows {
    param(
        [int]$TrialNumber,
        [string]$LowGapCsvPath
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

function Read-WeakestLowWorkloadRows {
    param(
        [int]$TrialNumber,
        [string]$WorkloadsCsvPath
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
            WorkloadName             = Get-WorkloadText -Row $WeakestRow -FieldName "workload_name"
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

function New-SummaryRow {
    param(
        [string]$BaselineName,
        [object[]]$TrialRows,
        [object[]]$PreviousRows
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

function New-WorkloadSummaryRow {
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

Set-Utf8Text -Path $TrialLogPath -Text "Repeated low-selectivity gap trials`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "===================================`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Trials: $Trials`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Iterations: $Iterations`r`n"

$PreviousCsvLoad = Get-CsvLoadStatus -Path $PreviousLowSelectivityGapCsv
$AllTrialRows = @()
$AllWorkloadRows = @()

for ($TrialNumber = 1; $TrialNumber -le $Trials; $TrialNumber++) {
    $TrialSuffix = $TrialNumber.ToString("000")
    $TrialLowGapCsvPath = Join-Path $TrialOutputDir "low-selectivity-gap-$Label-trial-$TrialSuffix.csv"
    $TrialWorkloadsCsvPath = Join-Path $TrialOutputDir "workloads-$Label-trial-$TrialSuffix.csv"

    Write-Host ""
    Write-Host "Running low-gap trial $TrialNumber of $Trials"
    Write-Host "  $TrialLowGapCsvPath"
    Write-Host "  $TrialWorkloadsCsvPath"

    Invoke-LowGapTrial `
        -TrialNumber $TrialNumber `
        -LowGapCsvPath $TrialLowGapCsvPath `
        -WorkloadsCsvPath $TrialWorkloadsCsvPath

    $AllTrialRows += Read-TrialRows `
        -TrialNumber $TrialNumber `
        -LowGapCsvPath $TrialLowGapCsvPath

    $AllWorkloadRows += Read-WeakestLowWorkloadRows `
        -TrialNumber $TrialNumber `
        -WorkloadsCsvPath $TrialWorkloadsCsvPath
}

$DetailRows = @()

foreach ($Row in $AllTrialRows) {
    $DetailRows += , @(
        $Row.Trial,
        $Row.BaselineName,
        $(Format-InvariantDouble -Value $Row.LowMeanTimingRatio),
        $(Format-InvariantDouble -Value $Row.LowWeightedCandidateRatio),
        $(Format-InvariantDouble -Value $Row.LowWeightedAvoidanceRatio),
        $Row.Status,
        $Row.SourceCsv
    )
}

Write-CsvDocument `
    -Path $DetailCsvPath `
    -Header @(
    "trial",
    "baseline_name",
    "low_mean_timing_ratio",
    "low_weighted_candidate_ratio",
    "low_weighted_avoidance_ratio",
    "status",
    "source_csv"
) `
    -Rows $DetailRows

$WorkloadDetailRows = @()

foreach ($Row in $AllWorkloadRows) {
    $WorkloadDetailRows += , @(
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
        $(Format-InvariantDouble -Value $Row.CandidateRatio),
        $(Format-InvariantDouble -Value $Row.NodesPerRecord),
        $Row.Status,
        $Row.SourceCsv
    )
}

Write-CsvDocument `
    -Path $WorkloadDetailCsvPath `
    -Header @(
    "trial",
    "baseline_name",
    "weakest_workload_name",
    "average_timing_ratio",
    "baseline_average_elapsed_ns",
    "fse_average_elapsed_ns",
    "fse_visited_nodes",
    "fse_retained_leaves",
    "fse_reconstructed_records",
    "fse_matched_records",
    "baseline_evaluated_records",
    "candidate_ratio",
    "nodes_per_record",
    "status",
    "source_csv"
) `
    -Rows $WorkloadDetailRows

$SummaryObjects = @()

foreach ($BaselineName in $TreeBaselines) {
    $SummaryObjects += New-SummaryRow `
        -BaselineName $BaselineName `
        -TrialRows $AllTrialRows `
        -PreviousRows $PreviousCsvLoad.Rows
}

$SummaryRows = @()

foreach ($Summary in $SummaryObjects) {
    $SummaryRows += , @(
        $Summary.BaselineName,
        $Summary.TrialCount,
        $(Format-InvariantDouble -Value $Summary.MeanLowMeanTimingRatio),
        $(Format-InvariantDouble -Value $Summary.MinLowMeanTimingRatio),
        $(Format-InvariantDouble -Value $Summary.MaxLowMeanTimingRatio),
        $(Format-InvariantDouble -Value $Summary.RangeLowMeanTimingRatio),
        $(Format-InvariantDouble -Value $Summary.SampleStdDevLowMeanTimingRatio),
        $(Format-InvariantDouble -Value $Summary.PreviousLowMeanTimingRatio),
        $(Format-InvariantDouble -Value $Summary.DeltaFromPrevious),
        $Summary.Classification,
        $(Format-InvariantDouble -Value $Summary.MeanLowWeightedCandidateRatio),
        $(Format-InvariantDouble -Value $Summary.MeanLowWeightedAvoidanceRatio)
    )
}

Write-CsvDocument `
    -Path $SummaryCsvPath `
    -Header @(
    "baseline_name",
    "trial_count",
    "mean_low_mean_timing_ratio",
    "min_low_mean_timing_ratio",
    "max_low_mean_timing_ratio",
    "range_low_mean_timing_ratio",
    "sample_stddev_low_mean_timing_ratio",
    "previous_low_mean_timing_ratio",
    "delta_from_previous",
    "classification",
    "mean_low_weighted_candidate_ratio",
    "mean_low_weighted_avoidance_ratio"
) `
    -Rows $SummaryRows

$WorkloadSummaryObjects = @()

foreach ($BaselineName in $TreeBaselines) {
    $WorkloadSummaryObjects += New-WorkloadSummaryRow `
        -BaselineName $BaselineName `
        -WorkloadRows $AllWorkloadRows
}

$WorkloadSummaryRows = @()

foreach ($Summary in $WorkloadSummaryObjects) {
    $WorkloadSummaryRows += , @(
        $Summary.BaselineName,
        $Summary.TrialCount,
        $Summary.MostFrequentWeakestWorkload,
        $Summary.WeakestWorkloadSequence,
        $(Format-InvariantDouble -Value $Summary.MeanWeakestTimingRatio),
        $(Format-InvariantDouble -Value $Summary.MinWeakestTimingRatio),
        $(Format-InvariantDouble -Value $Summary.MaxWeakestTimingRatio),
        $(Format-InvariantDouble -Value $Summary.RangeWeakestTimingRatio),
        $(Format-InvariantDouble -Value $Summary.SampleStdDevWeakestTimingRatio),
        $(Format-InvariantDouble -Value $Summary.MeanBaselineAverageElapsedNs),
        $(Format-InvariantDouble -Value $Summary.MeanFseAverageElapsedNs),
        $(Format-InvariantDouble -Value $Summary.MeanFseVisitedNodes),
        $(Format-InvariantDouble -Value $Summary.MeanFseRetainedLeaves),
        $(Format-InvariantDouble -Value $Summary.MeanFseReconstructedRecords),
        $(Format-InvariantDouble -Value $Summary.MeanBaselineEvaluatedRecords),
        $(Format-InvariantDouble -Value $Summary.MeanCandidateRatio),
        $(Format-InvariantDouble -Value $Summary.MeanNodesPerRecord)
    )
}

Write-CsvDocument `
    -Path $WorkloadSummaryCsvPath `
    -Header @(
    "baseline_name",
    "trial_count",
    "most_frequent_weakest_workload",
    "weakest_workload_sequence",
    "mean_weakest_timing_ratio",
    "min_weakest_timing_ratio",
    "max_weakest_timing_ratio",
    "range_weakest_timing_ratio",
    "sample_stddev_weakest_timing_ratio",
    "mean_baseline_average_elapsed_ns",
    "mean_fse_average_elapsed_ns",
    "mean_fse_visited_nodes",
    "mean_fse_retained_leaves",
    "mean_fse_reconstructed_records",
    "mean_baseline_evaluated_records",
    "mean_candidate_ratio",
    "mean_nodes_per_record"
) `
    -Rows $WorkloadSummaryRows

Set-Utf8Text -Path $NotesPath -Text "Repeated low-selectivity gap trial notes`r`n"
Add-Utf8Text -Path $NotesPath -Text "==========================================`r`n`r`n"
Add-Utf8Text -Path $NotesPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $NotesPath -Text "Trials: $Trials`r`n"
Add-Utf8Text -Path $NotesPath -Text "Iterations per trial: $Iterations`r`n"
Add-Utf8Text -Path $NotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $LowGapNoiseThreshold)`r`n"
Add-Utf8Text -Path $NotesPath -Text "Timing ratio meaning: baseline elapsed / FSE elapsed; higher is better for FSE.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Previous CSV: $PreviousLowSelectivityGapCsv`r`n"
Add-Utf8Text -Path $NotesPath -Text "Previous CSV status: $($PreviousCsvLoad.Status)`r`n"

if ($PreviousCsvLoad.Loaded) {
    Add-Utf8Text -Path $NotesPath -Text "Previous CSV resolved path: $($PreviousCsvLoad.ResolvedPath)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "Previous CSV rows loaded: $($PreviousCsvLoad.Rows.Count)`r`n"
}

Add-Utf8Text -Path $NotesPath -Text "`r`nLow-gap summary`r`n"
Add-Utf8Text -Path $NotesPath -Text "---------------`r`n"

foreach ($Summary in $SummaryObjects) {
    Add-Utf8Text -Path $NotesPath -Text "`r`n$($Summary.BaselineName)`r`n"
    Add-Utf8Text -Path $NotesPath -Text ("-" * $Summary.BaselineName.Length)
    Add-Utf8Text -Path $NotesPath -Text "`r`n"
    Add-Utf8Text -Path $NotesPath -Text "trials loaded: $($Summary.TrialCount)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean low timing: $(Format-InvariantDouble -Value $Summary.MeanLowMeanTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "min low timing: $(Format-InvariantDouble -Value $Summary.MinLowMeanTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "max low timing: $(Format-InvariantDouble -Value $Summary.MaxLowMeanTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "range: $(Format-InvariantDouble -Value $Summary.RangeLowMeanTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "sample stddev: $(Format-InvariantDouble -Value $Summary.SampleStdDevLowMeanTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "previous low timing: $(Format-InvariantDouble -Value $Summary.PreviousLowMeanTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "delta from previous: $(Format-InvariantDouble -Value $Summary.DeltaFromPrevious)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "classification: $($Summary.Classification)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean weighted candidate: $(Format-InvariantDouble -Value $Summary.MeanLowWeightedCandidateRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean weighted avoidance: $(Format-InvariantDouble -Value $Summary.MeanLowWeightedAvoidanceRatio)`r`n"
}

Add-Utf8Text -Path $NotesPath -Text "`r`nWeakest workload summary`r`n"
Add-Utf8Text -Path $NotesPath -Text "------------------------`r`n"

foreach ($Summary in $WorkloadSummaryObjects) {
    Add-Utf8Text -Path $NotesPath -Text "`r`n$($Summary.BaselineName)`r`n"
    Add-Utf8Text -Path $NotesPath -Text ("-" * $Summary.BaselineName.Length)
    Add-Utf8Text -Path $NotesPath -Text "`r`n"
    Add-Utf8Text -Path $NotesPath -Text "trials loaded: $($Summary.TrialCount)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "most frequent weakest workload: $($Summary.MostFrequentWeakestWorkload)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "weakest workload sequence: $($Summary.WeakestWorkloadSequence)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean weakest timing: $(Format-InvariantDouble -Value $Summary.MeanWeakestTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "min weakest timing: $(Format-InvariantDouble -Value $Summary.MinWeakestTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "max weakest timing: $(Format-InvariantDouble -Value $Summary.MaxWeakestTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "range weakest timing: $(Format-InvariantDouble -Value $Summary.RangeWeakestTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "sample stddev weakest timing: $(Format-InvariantDouble -Value $Summary.SampleStdDevWeakestTimingRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean baseline average ns: $(Format-InvariantDouble -Value $Summary.MeanBaselineAverageElapsedNs)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE average ns: $(Format-InvariantDouble -Value $Summary.MeanFseAverageElapsedNs)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE visited nodes: $(Format-InvariantDouble -Value $Summary.MeanFseVisitedNodes)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE retained leaves: $(Format-InvariantDouble -Value $Summary.MeanFseRetainedLeaves)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean FSE reconstructed records: $(Format-InvariantDouble -Value $Summary.MeanFseReconstructedRecords)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean baseline evaluated records: $(Format-InvariantDouble -Value $Summary.MeanBaselineEvaluatedRecords)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean candidate ratio: $(Format-InvariantDouble -Value $Summary.MeanCandidateRatio)`r`n"
    Add-Utf8Text -Path $NotesPath -Text "mean nodes per record: $(Format-InvariantDouble -Value $Summary.MeanNodesPerRecord)`r`n"
}

Add-Utf8Text -Path $NotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $NotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $NotesPath -Text "Use this script before accepting another query performance optimization.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Prefer changes that move the repeated-trial mean beyond the noise threshold.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Use weakest-workload rows to confirm which low-selectivity workload is consistently the problem for the selected dataset.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Do not treat one noisy trial as proof that the boundary workload improved.`r`n"

Write-Host ""
Write-Host "Low-gap trial artifacts written:"
Write-Host "  Dataset: $Dataset"
Write-Host "  Max depth: $EffectiveMaxDepth"
Write-Host "  $TrialOutputDir"
Write-Host "  $TrialLogPath"
Write-Host "  $DetailCsvPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $WorkloadDetailCsvPath"
Write-Host "  $WorkloadSummaryCsvPath"
Write-Host "  $NotesPath"