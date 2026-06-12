function Get-BenchmarkReviewRunAggregateComparisonSkipReason {
  param(
    [object]$Context
  )

  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $Context.PreviousTrialSummaryCsv `
    -CurrentPath $Context.CurrentTrialSummaryCsv `
    -PreviousDescription "previous aggregate trial summary" `
    -CurrentDescription "current aggregate trial summary"
}

function Get-BenchmarkReviewRunWorkloadComparisonSkipReason {
  param(
    [object]$Context
  )

  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $Context.PreviousWorkloadSummaryCsv `
    -CurrentPath $Context.CurrentWorkloadSummaryCsv `
    -PreviousDescription "previous workload trial summary" `
    -CurrentDescription "current workload trial summary"
}

function Get-BenchmarkReviewRunCountOnlyComparisonSkipReason {
  param(
    [object]$Context
  )

  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $Context.PreviousCountOnlySummaryCsv `
    -CurrentPath $Context.CurrentCountOnlyWorkloadSummaryCsv `
    -PreviousDescription "previous count-only workload summary" `
    -CurrentDescription "current count-only workload summary"
}

function Get-BenchmarkReviewRunMaterializationComparisonSkipReason {
  param(
    [object]$Context
  )

  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $Context.PreviousMaterializationSummaryCsv `
    -CurrentPath $Context.CurrentMaterializationSummaryCsv `
    -PreviousDescription "previous materialization mode summary" `
    -CurrentDescription "current materialization mode summary"
}

function Get-BenchmarkReviewRunTargetComparisonSkipReason {
  param(
    [object]$Context
  )

  return Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $Context.PreviousTargetSummaryCsv `
    -CurrentPath $Context.CurrentTargetSummaryCsv `
    -PreviousDescription "previous target workload summary" `
    -CurrentDescription "current target workload summary" `
    -Disabled ([bool]$Context.SkipTargetWorkloadReview) `
    -DisabledReason "target workload review was disabled"
}

function New-BenchmarkReviewRunManifestArtifacts {
  param(
    [object]$Context,
    [string]$AggregateComparisonSkipReason,
    [string]$WorkloadComparisonSkipReason,
    [string]$CountOnlyComparisonSkipReason,
    [string]$MaterializationComparisonSkipReason,
    [string]$TargetComparisonSkipReason
  )

  $AggregateComparisonSkipped = ![string]::IsNullOrWhiteSpace($AggregateComparisonSkipReason)
  $WorkloadComparisonSkipped = ![string]::IsNullOrWhiteSpace($WorkloadComparisonSkipReason)
  $CountOnlyComparisonSkipped = ![string]::IsNullOrWhiteSpace($CountOnlyComparisonSkipReason)
  $MaterializationComparisonSkipped = ![string]::IsNullOrWhiteSpace($MaterializationComparisonSkipReason)
  $TargetSummarySkipped = [bool]$Context.SkipTargetWorkloadReview
  $TargetComparisonSkipped = ![string]::IsNullOrWhiteSpace($TargetComparisonSkipReason)

  $TargetSummarySkipReason = if ($TargetSummarySkipped) { "target workload review was disabled" } else { "" }

  return @(
    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.BenchmarkOutput `
      -Path (Join-Path $Context.OutputDir "benchmark-output-$($Context.Label).txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.DebugOutput `
      -Path (Join-Path $Context.OutputDir "debug-output-$($Context.Label).txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.SummaryCsv `
      -Path (Join-Path $Context.OutputDir "summary-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.WorkloadsCsv `
      -Path (Join-Path $Context.OutputDir "workloads-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadSummary `
      -Path (Join-Path $Context.OutputDir "count-only-workload-summary-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadNotes `
      -Path (Join-Path $Context.OutputDir "count-only-workload-summary-$($Context.Label).txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonCsv `
      -Path (Join-Path $Context.OutputDir "count-only-workload-summary-comparison-$($Context.Label).csv") `
      -Skipped $CountOnlyComparisonSkipped `
      -Note $CountOnlyComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonNotes `
      -Path (Join-Path $Context.OutputDir "count-only-workload-summary-comparison-$($Context.Label).txt") `
      -Skipped $CountOnlyComparisonSkipped `
      -Note $CountOnlyComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.MaterializationModeSummary `
      -Path $Context.CurrentMaterializationSummaryCsv `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.MaterializationModeNotes `
      -Path $Context.CurrentMaterializationSummaryNotes `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TypedArchiveLoadSummary `
      -Path $Context.CurrentTypedArchiveLoadSummaryCsv `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TypedArchiveLoadNotes `
      -Path $Context.CurrentTypedArchiveLoadSummaryNotes `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TypedArchiveAppendRebuildSummary `
      -Path $Context.CurrentTypedArchiveAppendRebuildSummaryCsv `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TypedArchiveAppendRebuildNotes `
      -Path $Context.CurrentTypedArchiveAppendRebuildSummaryNotes `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TypedQueryIndexArchive `
      -Path $Context.CurrentTypedQueryIndexArchive `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.MaterializationModeComparisonCsv `
      -Path $Context.CurrentMaterializationComparisonCsv `
      -Skipped $MaterializationComparisonSkipped `
      -Note $MaterializationComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.MaterializationModeComparisonNotes `
      -Path $Context.CurrentMaterializationComparisonNotes `
      -Skipped $MaterializationComparisonSkipped `
      -Note $MaterializationComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowSelectivityGapCsv `
      -Path (Join-Path $Context.OutputDir "low-selectivity-gap-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapRegressionNotes `
      -Path (Join-Path $Context.OutputDir "low-gap-regression-notes-$($Context.Label).txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapTrialDetails `
      -Path (Join-Path $Context.OutputDir "low-gap-trial-details-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapTrialSummary `
      -Path (Join-Path $Context.OutputDir "low-gap-trial-summary-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapTrialNotes `
      -Path (Join-Path $Context.OutputDir "low-gap-trial-notes-$($Context.Label).txt") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapWorkloadTrialDetails `
      -Path (Join-Path $Context.OutputDir "low-gap-workload-trial-details-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapWorkloadTrialSummary `
      -Path (Join-Path $Context.OutputDir "low-gap-workload-trial-summary-$($Context.Label).csv") `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.AggregateTrialComparisonCsv `
      -Path (Join-Path $Context.OutputDir "low-gap-trial-comparison-$($Context.Label).csv") `
      -Skipped $AggregateComparisonSkipped `
      -Note $AggregateComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.AggregateTrialComparisonNotes `
      -Path (Join-Path $Context.OutputDir "low-gap-trial-comparison-$($Context.Label).txt") `
      -Skipped $AggregateComparisonSkipped `
      -Note $AggregateComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.WorkloadTrialComparisonCsv `
      -Path (Join-Path $Context.OutputDir "low-gap-workload-trial-comparison-$($Context.Label).csv") `
      -Skipped $WorkloadComparisonSkipped `
      -Note $WorkloadComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.WorkloadTrialComparisonNotes `
      -Path (Join-Path $Context.OutputDir "low-gap-workload-trial-comparison-$($Context.Label).txt") `
      -Skipped $WorkloadComparisonSkipped `
      -Note $WorkloadComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialDetails `
      -Path (Join-Path $Context.OutputDir "target-workload-trial-details-$($Context.Label).csv") `
      -Skipped $TargetSummarySkipped `
      -Note $TargetSummarySkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialSummary `
      -Path (Join-Path $Context.OutputDir "target-workload-trial-summary-$($Context.Label).csv") `
      -Skipped $TargetSummarySkipped `
      -Note $TargetSummarySkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialNotes `
      -Path (Join-Path $Context.OutputDir "target-workload-trial-notes-$($Context.Label).txt") `
      -Skipped $TargetSummarySkipped `
      -Note $TargetSummarySkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonCsv `
      -Path (Join-Path $Context.OutputDir "target-workload-trial-comparison-$($Context.Label).csv") `
      -Skipped $TargetComparisonSkipped `
      -Note $TargetComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonNotes `
      -Path (Join-Path $Context.OutputDir "target-workload-trial-comparison-$($Context.Label).txt") `
      -Skipped $TargetComparisonSkipped `
      -Note $TargetComparisonSkipReason

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapReviewNotes `
      -Path $Context.ReviewNotesPath `
      -Skipped $false

    New-BenchmarkReviewManifestArtifact `
      -Name $BenchmarkReviewArtifactNames.LowGapReviewManifest `
      -Path $Context.ReviewManifestPath `
      -Skipped $false
  )
}

function Write-BenchmarkReviewRunManifest {
  param(
    [object]$Context
  )

  $AggregateComparisonSkipReason = Get-BenchmarkReviewRunAggregateComparisonSkipReason -Context $Context
  $WorkloadComparisonSkipReason = Get-BenchmarkReviewRunWorkloadComparisonSkipReason -Context $Context
  $CountOnlyComparisonSkipReason = Get-BenchmarkReviewRunCountOnlyComparisonSkipReason -Context $Context
  $MaterializationComparisonSkipReason = Get-BenchmarkReviewRunMaterializationComparisonSkipReason -Context $Context
  $TargetComparisonSkipReason = Get-BenchmarkReviewRunTargetComparisonSkipReason -Context $Context

  Write-BenchmarkReviewManifestHeader `
    -ManifestPath $Context.ReviewManifestPath `
    -Label $Context.Label `
    -RunId $Context.RunId `
    -AttemptId $Context.AttemptId `
    -RunTopic $Context.RunTopic `
    -PreviousLabel $Context.PreviousLabel `
    -Dataset $Context.Dataset `
    -OrganizedRunRoot $Context.OrganizedRunRoot `
    -PreviousOrganizedRunDirectory $Context.PreviousOrganizedRunDirectory `
    -MaxDepth $Context.EffectiveMaxDepth `
    -Trials $Context.Trials `
    -Iterations $Context.Iterations `
    -TargetWorkloadName $Context.TargetWorkloadName `
    -CopyArtifactsToRunFolder ([bool]$Context.CopyArtifactsToRunFolder) `
    -ForceOrganizedArtifacts ([bool]$Context.ForceOrganizedArtifacts) `
    -ValidateArtifacts ([bool]$Context.ValidateArtifacts) `
    -RequireValidatedComparisons ([bool]$Context.RequireValidatedComparisons) `
    -CleanupFlatArtifacts ([bool]$Context.CleanupFlatArtifacts) `
    -UpdateHistory ([bool]$Context.UpdateHistory) `
    -HistoryDir $Context.HistoryDir `
    -NoiseThresholdText (Format-InvariantDouble -Value $Context.NoiseThreshold)

  $ManifestArtifacts = New-BenchmarkReviewRunManifestArtifacts `
    -Context $Context `
    -AggregateComparisonSkipReason $AggregateComparisonSkipReason `
    -WorkloadComparisonSkipReason $WorkloadComparisonSkipReason `
    -CountOnlyComparisonSkipReason $CountOnlyComparisonSkipReason `
    -MaterializationComparisonSkipReason $MaterializationComparisonSkipReason `
    -TargetComparisonSkipReason $TargetComparisonSkipReason

  Add-BenchmarkReviewManifestArtifacts `
    -ManifestPath $Context.ReviewManifestPath `
    -Artifacts $ManifestArtifacts

  Add-BenchmarkReviewManifestInterpretation -ManifestPath $Context.ReviewManifestPath
}
