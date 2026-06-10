function Initialize-BenchmarkReviewNotes {
  param(
    [string]$ReviewNotesPath,
    [string]$Label,
    [string]$RunId,
    [string]$AttemptId,
    [string]$RunTopic,
    [string]$PreviousLabel,
    [string]$Dataset,
    [int]$MaxDepth,
    [int]$Trials,
    [int]$Iterations,
    [string]$OutputDir,
    [string]$OrganizedRunRoot,
    [string]$PreviousOrganizedRunDirectory,
    [double]$NoiseThreshold,
    [bool]$SkipTargetWorkloadReview,
    [string]$TargetWorkloadName,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$ForceOrganizedArtifacts,
    [bool]$ValidateArtifacts,
    [bool]$RequireValidatedComparisons,
    [bool]$CleanupFlatArtifacts,
    [bool]$UpdateHistory,
    [string]$HistoryDir,
    [string]$PreviousLowSelectivityGapCsv,
    [string]$PreviousTrialSummaryCsv,
    [string]$PreviousWorkloadSummaryCsv,
    [string]$PreviousCountOnlySummaryCsv,
    [string]$PreviousMaterializationSummaryCsv,
    [string]$PreviousTargetSummaryCsv,
    [object]$PreviousLowSelectivityGapInput,
    [object]$PreviousTrialSummaryInput,
    [object]$PreviousWorkloadSummaryInput,
    [object]$PreviousCountOnlySummaryInput,
    [object]$PreviousMaterializationSummaryInput,
    [object]$PreviousTargetSummaryInput
  )

  Set-Utf8Text -Path $ReviewNotesPath -Text "Low-gap benchmark review`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "========================`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Label: $Label`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Run ID: $RunId`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Attempt ID: $AttemptId`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Run topic: $RunTopic`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous label: $PreviousLabel`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Max depth: $MaxDepth`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Trials: $Trials`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Iterations: $Iterations`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Output directory: $OutputDir`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Organized run root: $OrganizedRunRoot`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous organized run directory: $PreviousOrganizedRunDirectory`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Noise threshold: +/- $(Format-InvariantDouble -Value $NoiseThreshold)`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Target workload review skipped: $SkipTargetWorkloadReview`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Target workload name: $TargetWorkloadName`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Copy artifacts to run folder: $CopyArtifactsToRunFolder`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Force organized artifacts: $ForceOrganizedArtifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Validate artifacts: $ValidateArtifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Require validated comparisons: $RequireValidatedComparisons`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Cleanup flat artifacts: $CleanupFlatArtifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Update history: $UpdateHistory`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "History directory: $HistoryDir`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous low-selectivity gap CSV: $PreviousLowSelectivityGapCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous aggregate trial summary CSV: $PreviousTrialSummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous workload trial summary CSV: $PreviousWorkloadSummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous count-only workload summary CSV: $PreviousCountOnlySummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous materialization mode summary CSV: $PreviousMaterializationSummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Previous target workload summary CSV: $PreviousTargetSummaryCsv`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"

  Add-BenchmarkReviewPreviousInputStatusLines `
    -ReviewNotesPath $ReviewNotesPath `
    -SingleRunInput $PreviousLowSelectivityGapInput `
    -AggregateInput $PreviousTrialSummaryInput `
    -WorkloadInput $PreviousWorkloadSummaryInput `
    -TargetInput $PreviousTargetSummaryInput

  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary status: $($PreviousCountOnlySummaryInput.Status)`r`n"

  if ($PreviousCountOnlySummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary resolved path: $($PreviousCountOnlySummaryInput.ResolvedPath)`r`n"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary status: $($PreviousMaterializationSummaryInput.Status)`r`n"

  if ($PreviousMaterializationSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary resolved path: $($PreviousMaterializationSummaryInput.ResolvedPath)`r`n"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
}

function Write-BenchmarkReviewManifestHeader {
  param(
    [string]$ManifestPath,
    [string]$Label,
    [string]$RunId,
    [string]$AttemptId,
    [string]$RunTopic,
    [string]$PreviousLabel,
    [string]$Dataset,
    [string]$OrganizedRunRoot,
    [string]$PreviousOrganizedRunDirectory,
    [string]$MaxDepth,
    [int]$Trials,
    [int]$Iterations,
    [string]$TargetWorkloadName,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$ForceOrganizedArtifacts,
    [bool]$ValidateArtifacts,
    [bool]$RequireValidatedComparisons,
    [bool]$CleanupFlatArtifacts,
    [bool]$UpdateHistory,
    [string]$HistoryDir,
    [string]$NoiseThresholdText
  )

  Set-Utf8Text -Path $ManifestPath -Text "Low-gap review artifact manifest`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "================================`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Label: $Label`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Run ID: $RunId`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Attempt ID: $AttemptId`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Run topic: $RunTopic`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Previous label: $PreviousLabel`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Organized run root: $OrganizedRunRoot`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Previous organized run directory: $PreviousOrganizedRunDirectory`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Max depth: $MaxDepth`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Trials: $Trials`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Iterations: $Iterations`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Target workload: $TargetWorkloadName`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Copy artifacts to run folder: $CopyArtifactsToRunFolder`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Force organized artifacts: $ForceOrganizedArtifacts`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Validate artifacts: $ValidateArtifacts`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Require validated comparisons: $RequireValidatedComparisons`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Cleanup flat artifacts: $CleanupFlatArtifacts`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Update history: $UpdateHistory`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "History directory: $HistoryDir`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Noise threshold: +/- $NoiseThresholdText`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "state | artifact | path | note`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "------|----------|------|-----`r`n"
}

function Add-BenchmarkReviewManifestInterpretation {
  param(
    [string]$ManifestPath
  )

  Add-Utf8Text -Path $ManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Manifest interpretation`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "-----------------------`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "found: the artifact exists after the review run`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "missing: the artifact was expected but was not found`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "skipped: the artifact was intentionally not generated because an input or flag made that step unavailable`r`n"
}

function Add-BenchmarkReviewArtifactSummary {
  param(
    [string]$ReviewNotesPath,
    [string]$OutputDir,
    [string]$Label,
    [string]$ReviewManifestPath,
    [string]$TargetWorkloadName
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Review artifacts`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-----------------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "benchmark output: $(Join-Path $OutputDir "benchmark-output-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "debug output: $(Join-Path $OutputDir "debug-output-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "summary CSV: $(Join-Path $OutputDir "summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "workloads CSV: $(Join-Path $OutputDir "workloads-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary: $(Join-Path $OutputDir "count-only-workload-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload notes: $(Join-Path $OutputDir "count-only-workload-summary-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload comparison CSV: $(Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload comparison notes: $(Join-Path $OutputDir "count-only-workload-summary-comparison-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary: $(Join-Path $OutputDir "materialization-mode-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode notes: $(Join-Path $OutputDir "materialization-mode-summary-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "typed archive load summary: $(Join-Path $OutputDir "typed-archive-load-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "typed archive load notes: $(Join-Path $OutputDir "typed-archive-load-summary-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "typed query index archive: $(Join-Path $OutputDir "typed-query-index-archive-$Label.fse")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode comparison CSV: $(Join-Path $OutputDir "materialization-mode-summary-comparison-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode comparison notes: $(Join-Path $OutputDir "materialization-mode-summary-comparison-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-selectivity gap CSV: $(Join-Path $OutputDir "low-selectivity-gap-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap regression notes: $(Join-Path $OutputDir "low-gap-regression-notes-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap trial details: $(Join-Path $OutputDir "low-gap-trial-details-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap trial summary: $(Join-Path $OutputDir "low-gap-trial-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap trial notes: $(Join-Path $OutputDir "low-gap-trial-notes-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap workload trial details: $(Join-Path $OutputDir "low-gap-workload-trial-details-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap workload trial summary: $(Join-Path $OutputDir "low-gap-workload-trial-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial details: $(Join-Path $OutputDir "target-workload-trial-details-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial summary: $(Join-Path $OutputDir "target-workload-trial-summary-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial notes: $(Join-Path $OutputDir "target-workload-trial-notes-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial comparison CSV: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.csv")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "target workload trial comparison notes: $(Join-Path $OutputDir "target-workload-trial-comparison-$Label.txt")`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap review notes: $ReviewNotesPath`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "low-gap review manifest: $ReviewManifestPath`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Decision guidance`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-----------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use this review script for query-performance commits.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use aggregate comparison artifacts for low-selectivity bucket movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use workload comparison artifacts for weakest-workload movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use count-only workload comparison artifacts for count-only query mode movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use materialization mode comparison artifacts for output-contract movement across fresh owned, reusable owned, reference-result, reference-visitor, row-view, and count-only paths.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Use target-workload comparison artifacts for $TargetWorkloadName movement.`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Do not accept a performance commit based on one noisy single-run result.`r`n"
}

function Add-BenchmarkReviewComparisonAvailabilitySummary {
  param(
    [string]$ReviewNotesPath,
    [object]$PreviousLowSelectivityGapInput,
    [object]$PreviousTrialSummaryInput,
    [string]$CurrentTrialSummaryCsv,
    [object]$PreviousWorkloadSummaryInput,
    [string]$CurrentWorkloadSummaryCsv,
    [object]$PreviousCountOnlySummaryInput,
    [string]$CurrentCountOnlyWorkloadSummaryCsv,
    [object]$PreviousMaterializationSummaryInput,
    [string]$CurrentMaterializationSummaryCsv,
    [bool]$SkipTargetWorkloadReview,
    [object]$PreviousTargetSummaryInput,
    [string]$CurrentTargetSummaryCsv
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Comparison availability summary`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-------------------------------`r`n"

  if ($PreviousLowSelectivityGapInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-gap comparison: completed`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-gap comparison: unavailable`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "single-run low-gap comparison reason: previous low-selectivity gap CSV was $($PreviousLowSelectivityGapInput.Status)`r`n"
  }

  if ($PreviousTrialSummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentTrialSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison: completed`r`n"
  }
  elseif (!$PreviousTrialSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison reason: previous aggregate trial summary was $($PreviousTrialSummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "aggregate repeated-trial comparison reason: current aggregate trial summary was missing`r`n"
  }

  if ($PreviousWorkloadSummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentWorkloadSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison: completed`r`n"
  }
  elseif (!$PreviousWorkloadSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison reason: previous workload trial summary was $($PreviousWorkloadSummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "workload repeated-trial comparison reason: current workload trial summary was missing`r`n"
  }

  if ($PreviousCountOnlySummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentCountOnlyWorkloadSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison: completed`r`n"
  }
  elseif (!$PreviousCountOnlySummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison reason: previous count-only workload summary was $($PreviousCountOnlySummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "count-only workload summary comparison reason: current count-only workload summary was missing`r`n"
  }

  if ($PreviousMaterializationSummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentMaterializationSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary comparison: completed`r`n"
  }
  elseif (!$PreviousMaterializationSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary comparison reason: previous materialization mode summary was $($PreviousMaterializationSummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "materialization mode summary comparison reason: current materialization mode summary was missing`r`n"
  }

  if ($SkipTargetWorkloadReview) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary reason: target workload review was disabled`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison reason: target workload review was disabled`r`n"
    return
  }

  if (Test-BenchmarkReviewPathExists -Path $CurrentTargetSummaryCsv) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary: completed`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary: unavailable`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload summary reason: current target workload summary was missing`r`n"
  }

  if ($PreviousTargetSummaryInput.Exists -and (Test-BenchmarkReviewPathExists -Path $CurrentTargetSummaryCsv)) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: completed`r`n"
  }
  elseif (!$PreviousTargetSummaryInput.Exists) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison reason: previous target workload summary was $($PreviousTargetSummaryInput.Status)`r`n"
  }
  else {
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "target workload comparison reason: current target workload summary was missing`r`n"
  }
}

function Write-BenchmarkReviewCompletionOutput {
  param(
    [string]$ReviewNotesPath,
    [string]$ReviewManifestPath,
    [string]$OrganizedRunDirectory,
    [bool]$CopyArtifactsToRunFolder,
    [bool]$CleanupFlatArtifacts,
    [bool]$UpdateHistory,
    [string]$HistoryDir
  )

  Write-Host ""
  Write-Host "Low-gap review complete:"

  if ($CleanupFlatArtifacts -and $CopyArtifactsToRunFolder) {
    Write-Host "  organized review notes: $(Join-Path $OrganizedRunDirectory (Split-Path -Leaf $ReviewNotesPath))"
    Write-Host "  organized review manifest: $(Join-Path $OrganizedRunDirectory (Split-Path -Leaf $ReviewManifestPath))"
    Write-Host "  organized run folder: $OrganizedRunDirectory"
  }
  else {
    Write-Host "  $ReviewNotesPath"
    Write-Host "  $ReviewManifestPath"

    if ($CopyArtifactsToRunFolder) {
      Write-Host "  organized run folder: $OrganizedRunDirectory"
    }
  }

  if ($UpdateHistory) {
    Write-Host "  history folder: $HistoryDir"
  }
}
