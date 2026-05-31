param(
  [string]$Label = "",
  [ValidateRange(0, 2147483647)]
  [int]$RunNumber = 0,
  [ValidateRange(1, 2147483647)]
  [int]$RunIndex = 1,
  [string]$RunTopic = "",
  [ValidateSet("small", "large")]
  [string]$Dataset = "small",
  [ValidateRange(0, 2147483647)]
  [int]$MaxDepth = 0,
  [ValidateRange(1, 2147483647)]
  [int]$Trials = 5,
  [int]$Iterations = 10000,
  [string]$OutputDir = "benchmark_artifacts"
)

$ErrorActionPreference = "Stop"

$ScriptsRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BenchmarkScriptRoot = Split-Path -Parent $PSScriptRoot

$BenchmarkCsvLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-csv.ps1"
$BenchmarkReviewManifestLibrary = Join-Path (Join-Path $ScriptsRoot "lib") "benchmark-review-manifest.ps1"

if (!(Test-Path -LiteralPath $BenchmarkCsvLibrary)) {
  throw "benchmark CSV helper library was not found: $BenchmarkCsvLibrary"
}

if (!(Test-Path -LiteralPath $BenchmarkReviewManifestLibrary)) {
  throw "benchmark review manifest helper library was not found: $BenchmarkReviewManifestLibrary"
}

. $BenchmarkCsvLibrary
. $BenchmarkReviewManifestLibrary

$Label = Resolve-BenchmarkReviewLabel `
  -Label $Label `
  -RunNumber $RunNumber `
  -RunIndex $RunIndex `
  -RunTopic $RunTopic `
  -Dataset $Dataset

$RunId = ""
$AttemptId = ""
$NormalizedRunTopic = ""

if ($RunNumber -gt 0) {
  $RunId = Format-BenchmarkReviewRunId -RunNumber $RunNumber
  $AttemptId = Format-BenchmarkReviewAttemptId -RunIndex $RunIndex
  $NormalizedRunTopic = Normalize-BenchmarkReviewRunTopic -RunTopic $RunTopic
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
  throw "-OutputDir cannot be empty"
}

if ($Iterations -lt 1) {
  throw "-Iterations must be at least 1"
}

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
$MaterializationSummaryScript = Join-Path (Join-Path $BenchmarkScriptRoot "summarize") "summarize-materialization-mode-summary.ps1"

if (!(Test-Path -LiteralPath $MaterializationSummaryScript)) {
  throw "required script was not found: $MaterializationSummaryScript"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TrialOutputDir = Join-Path $OutputDir "materialization-mode-trials-$Label"
New-Item -ItemType Directory -Force -Path $TrialOutputDir | Out-Null

$TrialLogPath = Join-Path $TrialOutputDir "materialization-mode-trial-output-$Label.txt"
$DetailCsvPath = Join-Path $OutputDir "materialization-mode-trial-details-$Label.csv"
$SummaryCsvPath = Join-Path $OutputDir "materialization-mode-trial-summary-$Label.csv"
$NotesPath = Join-Path $OutputDir "materialization-mode-trial-notes-$Label.txt"

function Convert-MaterializationDurationToNanoseconds {
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

  if ($LowerTextValue -eq "unavailable" -or $LowerTextValue -eq "not available" -or $LowerTextValue -eq "n/a" -or $LowerTextValue -eq "na") {
    return $null
  }

  $Match = [Regex]::Match($LowerTextValue, "^([0-9]+(?:\.[0-9]+)?)(ns|us|ms|s)$")

  if (!$Match.Success) {
    throw "unsupported duration value in materialization trial summary: $Value"
  }

  $Amount = [double]::Parse($Match.Groups[1].Value, $BenchmarkInvariantCulture)
  $Unit = $Match.Groups[2].Value

  switch ($Unit) {
    "ns" { return $Amount }
    "us" { return $Amount * 1000.0 }
    "ms" { return $Amount * 1000000.0 }
    "s" { return $Amount * 1000000000.0 }
    default { throw "unsupported duration unit in materialization trial summary: $Unit" }
  }
}

function Convert-MaterializationCount {
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

  return [System.Convert]::ToInt64($TextValue, $BenchmarkInvariantCulture)
}

function Get-PropertyValues {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return @(
    $Rows |
    Where-Object { $null -ne $_.$PropertyName } |
    ForEach-Object { [double]$_.$PropertyName }
  )
}

function Get-PropertyMean {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return Get-Mean -Values (Get-PropertyValues -Rows $Rows -PropertyName $PropertyName)
}

function Get-PropertyMinimum {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return Get-Minimum -Values (Get-PropertyValues -Rows $Rows -PropertyName $PropertyName)
}

function Get-PropertyMaximum {
  param(
    [object[]]$Rows,
    [string]$PropertyName
  )

  return Get-Maximum -Values (Get-PropertyValues -Rows $Rows -PropertyName $PropertyName)
}

function Get-PropertyStdDev {
  param(
    [object[]]$Rows,
    [string]$PropertyName,
    [object]$Mean
  )

  return Get-SampleStdDev `
    -Values (Get-PropertyValues -Rows $Rows -PropertyName $PropertyName) `
    -Mean $Mean
}

function Get-FasterTrialCount {
  param(
    [object[]]$Rows,
    [string]$LeftPropertyName,
    [string]$RightPropertyName
  )

  $Count = 0

  foreach ($Row in $Rows) {
    $LeftValue = $Row.$LeftPropertyName
    $RightValue = $Row.$RightPropertyName

    if ($null -eq $LeftValue -or $null -eq $RightValue) {
      continue
    }

    if ([double]$LeftValue -lt [double]$RightValue) {
      $Count += 1
    }
  }

  return $Count
}

function New-MaterializationTrialDetailRow {
  param(
    [int]$TrialNumber,
    [object]$SourceRow,
    [string]$SourceCsv
  )

  return [PSCustomObject]@{
    Trial                  = $TrialNumber
    Dataset                = Get-TextField -Row $SourceRow -FieldName "dataset"
    MaxDepth               = Get-TextField -Row $SourceRow -FieldName "max_depth"
    WorkloadName           = Get-TextField -Row $SourceRow -FieldName "workload_name"
    MatchedRecords         = Convert-MaterializationCount -Value $SourceRow.matched_records
    FreshOwnedElapsedNs    = Convert-MaterializationDurationToNanoseconds -Value $SourceRow.fresh_owned_elapsed
    ReusableOwnedElapsedNs = Convert-MaterializationDurationToNanoseconds -Value $SourceRow.reusable_owned_elapsed
    ReferenceElapsedNs     = Convert-MaterializationDurationToNanoseconds -Value $SourceRow.reference_elapsed
    VisitorElapsedNs       = Convert-MaterializationDurationToNanoseconds -Value $SourceRow.visitor_elapsed
    RowViewElapsedNs       = Convert-MaterializationDurationToNanoseconds -Value $SourceRow.row_view_elapsed
    CountOnlyElapsedNs     = Convert-MaterializationDurationToNanoseconds -Value $SourceRow.count_only_elapsed
    CountSpeedup           = Convert-ToInvariantDouble -Value $SourceRow.count_speedup
    ReferenceSpeedup       = Convert-ToInvariantDouble -Value $SourceRow.reference_speedup
    VisitorSpeedup         = Convert-ToInvariantDouble -Value $SourceRow.visitor_speedup
    RowViewSpeedup         = Convert-ToInvariantDouble -Value $SourceRow.row_view_speedup
    ReusableSpeedup        = Convert-ToInvariantDouble -Value $SourceRow.reusable_speedup
    Agreement              = Get-LowerTextField -Row $SourceRow -FieldName "agreement"
    SourceCsv              = $SourceCsv
  }
}

function New-MaterializationTrialSummaryRow {
  param(
    [string]$WorkloadName,
    [object[]]$DetailRows
  )

  $Rows = @(
    $DetailRows |
    Where-Object { $_.WorkloadName -eq $WorkloadName }
  )

  $TrialCount = $Rows.Count
  $AgreementPassCount = @($Rows | Where-Object { $_.Agreement -eq "pass" }).Count
  $MatchedValues = @(
    $Rows |
    Where-Object { $null -ne $_.MatchedRecords } |
    ForEach-Object { $_.MatchedRecords } |
    Select-Object -Unique
  )

  if ($MatchedValues.Count -eq 1) {
    $MatchedRecords = $MatchedValues[0]
  }
  else {
    $MatchedRecords = "mixed"
  }

  $MeanRowViewElapsedNs = Get-PropertyMean -Rows $Rows -PropertyName "RowViewElapsedNs"
  $MinRowViewElapsedNs = Get-PropertyMinimum -Rows $Rows -PropertyName "RowViewElapsedNs"
  $MaxRowViewElapsedNs = Get-PropertyMaximum -Rows $Rows -PropertyName "RowViewElapsedNs"
  $RangeRowViewElapsedNs = $null

  if ($null -ne $MinRowViewElapsedNs -and $null -ne $MaxRowViewElapsedNs) {
    $RangeRowViewElapsedNs = $MaxRowViewElapsedNs - $MinRowViewElapsedNs
  }

  return [PSCustomObject]@{
    Dataset                              = $Rows[0].Dataset
    MaxDepth                             = $Rows[0].MaxDepth
    WorkloadName                         = $WorkloadName
    TrialCount                           = $TrialCount
    MatchedRecords                       = $MatchedRecords
    AgreementPassCount                   = $AgreementPassCount
    AllAgreementPass                     = ($TrialCount -gt 0 -and $AgreementPassCount -eq $TrialCount)
    MeanFreshOwnedElapsedNs              = Get-PropertyMean -Rows $Rows -PropertyName "FreshOwnedElapsedNs"
    MeanReusableOwnedElapsedNs           = Get-PropertyMean -Rows $Rows -PropertyName "ReusableOwnedElapsedNs"
    MeanReferenceElapsedNs               = Get-PropertyMean -Rows $Rows -PropertyName "ReferenceElapsedNs"
    MeanVisitorElapsedNs                 = Get-PropertyMean -Rows $Rows -PropertyName "VisitorElapsedNs"
    MeanRowViewElapsedNs                 = $MeanRowViewElapsedNs
    MeanCountOnlyElapsedNs               = Get-PropertyMean -Rows $Rows -PropertyName "CountOnlyElapsedNs"
    MinRowViewElapsedNs                  = $MinRowViewElapsedNs
    MaxRowViewElapsedNs                  = $MaxRowViewElapsedNs
    RangeRowViewElapsedNs                = $RangeRowViewElapsedNs
    SampleStdDevRowViewElapsedNs         = Get-PropertyStdDev -Rows $Rows -PropertyName "RowViewElapsedNs" -Mean $MeanRowViewElapsedNs
    MeanCountSpeedup                     = Get-PropertyMean -Rows $Rows -PropertyName "CountSpeedup"
    MeanReferenceSpeedup                 = Get-PropertyMean -Rows $Rows -PropertyName "ReferenceSpeedup"
    MeanVisitorSpeedup                   = Get-PropertyMean -Rows $Rows -PropertyName "VisitorSpeedup"
    MeanRowViewSpeedup                   = Get-PropertyMean -Rows $Rows -PropertyName "RowViewSpeedup"
    MeanReusableSpeedup                  = Get-PropertyMean -Rows $Rows -PropertyName "ReusableSpeedup"
    RowViewFasterThanFreshOwnedTrials    = Get-FasterTrialCount -Rows $Rows -LeftPropertyName "RowViewElapsedNs" -RightPropertyName "FreshOwnedElapsedNs"
    RowViewFasterThanReusableOwnedTrials = Get-FasterTrialCount -Rows $Rows -LeftPropertyName "RowViewElapsedNs" -RightPropertyName "ReusableOwnedElapsedNs"
    RowViewFasterThanReferenceTrials     = Get-FasterTrialCount -Rows $Rows -LeftPropertyName "RowViewElapsedNs" -RightPropertyName "ReferenceElapsedNs"
    RowViewFasterThanVisitorTrials       = Get-FasterTrialCount -Rows $Rows -LeftPropertyName "RowViewElapsedNs" -RightPropertyName "VisitorElapsedNs"
    RowViewFasterThanCountOnlyTrials     = Get-FasterTrialCount -Rows $Rows -LeftPropertyName "RowViewElapsedNs" -RightPropertyName "CountOnlyElapsedNs"
  }
}

function Invoke-MaterializationTrial {
  param(
    [int]$TrialNumber,
    [string]$DebugOutputPath
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
    "--debug-report"
  )

  Set-Utf8Text -Path $DebugOutputPath -Text "Materialization mode trial $TrialNumber`r`n"
  Add-Utf8Text -Path $DebugOutputPath -Text "================================`r`n"
  Add-Utf8Text -Path $DebugOutputPath -Text "Label: $Label`r`n"
  Add-Utf8Text -Path $DebugOutputPath -Text "Dataset: $Dataset`r`n"
  Add-Utf8Text -Path $DebugOutputPath -Text "Max depth: $EffectiveMaxDepth`r`n"
  Add-Utf8Text -Path $DebugOutputPath -Text "Iterations: $Iterations`r`n`r`n"

  Add-Utf8Text -Path $TrialLogPath -Text "`r`n---`r`n`r`nTrial $TrialNumber`r`n"
  Add-Utf8Text -Path $TrialLogPath -Text "Debug output: $DebugOutputPath`r`n"

  $CommandOutput = & cargo @CargoArguments 2>&1
  $ExitCode = $LASTEXITCODE

  foreach ($Line in $CommandOutput) {
    $LineText = $Line.ToString()
    Write-Host $LineText
    Add-Utf8Text -Path $DebugOutputPath -Text "$LineText`r`n"
    Add-Utf8Text -Path $TrialLogPath -Text "$LineText`r`n"
  }

  if ($ExitCode -ne 0) {
    throw "materialization mode trial $TrialNumber failed with exit code $ExitCode"
  }
}

Set-Utf8Text -Path $TrialLogPath -Text "Repeated materialization mode trials`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "====================================`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Run id: $RunId`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Attempt id: $AttemptId`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Run topic: $NormalizedRunTopic`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Max depth: $EffectiveMaxDepth`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Trials: $Trials`r`n"
Add-Utf8Text -Path $TrialLogPath -Text "Iterations: $Iterations`r`n"

$DetailObjects = @()

for ($TrialNumber = 1; $TrialNumber -le $Trials; $TrialNumber++) {
  $TrialSuffix = $TrialNumber.ToString("000")
  $TrialDebugOutputPath = Join-Path $TrialOutputDir "debug-output-$Label-trial-$TrialSuffix.txt"
  $TrialSummaryCsvPath = Join-Path $TrialOutputDir "materialization-mode-summary-$Label-trial-$TrialSuffix.csv"
  $TrialSummaryNotesPath = Join-Path $TrialOutputDir "materialization-mode-summary-$Label-trial-$TrialSuffix.txt"

  Write-Host ""
  Write-Host "Running materialization mode trial $TrialNumber of $Trials"
  Write-Host "  $TrialDebugOutputPath"

  Invoke-MaterializationTrial `
    -TrialNumber $TrialNumber `
    -DebugOutputPath $TrialDebugOutputPath

  & $MaterializationSummaryScript `
    -DebugOutputPath $TrialDebugOutputPath `
    -OutputCsv $TrialSummaryCsvPath `
    -OutputNotesPath $TrialSummaryNotesPath `
    -Dataset $Dataset `
    -MaxDepth $EffectiveMaxDepthText

  if ($LASTEXITCODE -ne 0) {
    throw "materialization summary extraction failed for trial $TrialNumber with exit code $LASTEXITCODE"
  }

  if (!(Test-Path -LiteralPath $TrialSummaryCsvPath)) {
    throw "materialization mode trial $TrialNumber did not write expected CSV: $TrialSummaryCsvPath"
  }

  $TrialRows = @(Import-Csv -LiteralPath $TrialSummaryCsvPath)

  foreach ($Row in $TrialRows) {
    $DetailObjects += New-MaterializationTrialDetailRow `
      -TrialNumber $TrialNumber `
      -SourceRow $Row `
      -SourceCsv $TrialSummaryCsvPath
  }
}

$DetailRows = @()

foreach ($Row in $DetailObjects) {
  $DetailRows += , @(
    $Row.Trial,
    $Row.Dataset,
    $Row.MaxDepth,
    $Row.WorkloadName,
    $Row.MatchedRecords,
    $(Format-InvariantDouble -Value $Row.FreshOwnedElapsedNs),
    $(Format-InvariantDouble -Value $Row.ReusableOwnedElapsedNs),
    $(Format-InvariantDouble -Value $Row.ReferenceElapsedNs),
    $(Format-InvariantDouble -Value $Row.VisitorElapsedNs),
    $(Format-InvariantDouble -Value $Row.RowViewElapsedNs),
    $(Format-InvariantDouble -Value $Row.CountOnlyElapsedNs),
    $(Format-InvariantDouble -Value $Row.CountSpeedup),
    $(Format-InvariantDouble -Value $Row.ReferenceSpeedup),
    $(Format-InvariantDouble -Value $Row.VisitorSpeedup),
    $(Format-InvariantDouble -Value $Row.RowViewSpeedup),
    $(Format-InvariantDouble -Value $Row.ReusableSpeedup),
    $Row.Agreement,
    $Row.SourceCsv
  )
}

Write-CsvDocument `
  -Path $DetailCsvPath `
  -Header @(
  "trial",
  "dataset",
  "max_depth",
  "workload_name",
  "matched_records",
  "fresh_owned_elapsed_ns",
  "reusable_owned_elapsed_ns",
  "reference_elapsed_ns",
  "visitor_elapsed_ns",
  "row_view_elapsed_ns",
  "count_only_elapsed_ns",
  "count_speedup",
  "reference_speedup",
  "visitor_speedup",
  "row_view_speedup",
  "reusable_speedup",
  "agreement",
  "source_csv"
) `
  -Rows $DetailRows

$SummaryObjects = @()
$WorkloadNames = @(
  $DetailObjects |
  ForEach-Object { $_.WorkloadName } |
  Select-Object -Unique |
  Sort-Object
)

foreach ($WorkloadName in $WorkloadNames) {
  $SummaryObjects += New-MaterializationTrialSummaryRow `
    -WorkloadName $WorkloadName `
    -DetailRows $DetailObjects
}

$SummaryRows = @()

foreach ($Summary in $SummaryObjects) {
  $SummaryRows += , @(
    $Summary.Dataset,
    $Summary.MaxDepth,
    $Summary.WorkloadName,
    $Summary.TrialCount,
    $Summary.MatchedRecords,
    $Summary.AgreementPassCount,
    $Summary.AllAgreementPass.ToString().ToLowerInvariant(),
    $(Format-InvariantDouble -Value $Summary.MeanFreshOwnedElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanVisitorElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanRowViewElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanCountOnlyElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MinRowViewElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MaxRowViewElapsedNs),
    $(Format-InvariantDouble -Value $Summary.RangeRowViewElapsedNs),
    $(Format-InvariantDouble -Value $Summary.SampleStdDevRowViewElapsedNs),
    $(Format-InvariantDouble -Value $Summary.MeanCountSpeedup),
    $(Format-InvariantDouble -Value $Summary.MeanReferenceSpeedup),
    $(Format-InvariantDouble -Value $Summary.MeanVisitorSpeedup),
    $(Format-InvariantDouble -Value $Summary.MeanRowViewSpeedup),
    $(Format-InvariantDouble -Value $Summary.MeanReusableSpeedup),
    $Summary.RowViewFasterThanFreshOwnedTrials,
    $Summary.RowViewFasterThanReusableOwnedTrials,
    $Summary.RowViewFasterThanReferenceTrials,
    $Summary.RowViewFasterThanVisitorTrials,
    $Summary.RowViewFasterThanCountOnlyTrials
  )
}

Write-CsvDocument `
  -Path $SummaryCsvPath `
  -Header @(
  "dataset",
  "max_depth",
  "workload_name",
  "trial_count",
  "matched_records",
  "agreement_pass_count",
  "all_agreement_pass",
  "mean_fresh_owned_elapsed_ns",
  "mean_reusable_owned_elapsed_ns",
  "mean_reference_elapsed_ns",
  "mean_visitor_elapsed_ns",
  "mean_row_view_elapsed_ns",
  "mean_count_only_elapsed_ns",
  "min_row_view_elapsed_ns",
  "max_row_view_elapsed_ns",
  "range_row_view_elapsed_ns",
  "sample_stddev_row_view_elapsed_ns",
  "mean_count_speedup",
  "mean_reference_speedup",
  "mean_visitor_speedup",
  "mean_row_view_speedup",
  "mean_reusable_speedup",
  "row_view_faster_than_fresh_owned_trials",
  "row_view_faster_than_reusable_owned_trials",
  "row_view_faster_than_reference_trials",
  "row_view_faster_than_visitor_trials",
  "row_view_faster_than_count_only_trials"
) `
  -Rows $SummaryRows

$AgreementFailures = @($DetailObjects | Where-Object { $_.Agreement -ne "pass" })
$ExpectedRows = $Trials * $SummaryObjects.Count

Set-Utf8Text -Path $NotesPath -Text "Repeated materialization mode trial notes`r`n"
Add-Utf8Text -Path $NotesPath -Text "==========================================`r`n`r`n"
Add-Utf8Text -Path $NotesPath -Text "Label: $Label`r`n"
Add-Utf8Text -Path $NotesPath -Text "Run id: $RunId`r`n"
Add-Utf8Text -Path $NotesPath -Text "Attempt id: $AttemptId`r`n"
Add-Utf8Text -Path $NotesPath -Text "Run topic: $NormalizedRunTopic`r`n"
Add-Utf8Text -Path $NotesPath -Text "Dataset: $Dataset`r`n"
Add-Utf8Text -Path $NotesPath -Text "Max depth: $EffectiveMaxDepth`r`n"
Add-Utf8Text -Path $NotesPath -Text "Trials: $Trials`r`n"
Add-Utf8Text -Path $NotesPath -Text "Iterations per trial: $Iterations`r`n"
Add-Utf8Text -Path $NotesPath -Text "Trial directory: $TrialOutputDir`r`n"
Add-Utf8Text -Path $NotesPath -Text "Trial debug log: $TrialLogPath`r`n"
Add-Utf8Text -Path $NotesPath -Text "Detail CSV: $DetailCsvPath`r`n"
Add-Utf8Text -Path $NotesPath -Text "Summary CSV: $SummaryCsvPath`r`n"
Add-Utf8Text -Path $NotesPath -Text "Workload rows: $($SummaryObjects.Count)`r`n"
Add-Utf8Text -Path $NotesPath -Text "Expected detail rows: $ExpectedRows`r`n"
Add-Utf8Text -Path $NotesPath -Text "Loaded detail rows: $($DetailObjects.Count)`r`n"
Add-Utf8Text -Path $NotesPath -Text "Agreement failures: $($AgreementFailures.Count)`r`n"

Add-Utf8Text -Path $NotesPath -Text "`r`nSummary rows`r`n"
Add-Utf8Text -Path $NotesPath -Text "------------`r`n"

foreach ($Summary in $SummaryObjects) {
  Add-Utf8Text -Path $NotesPath -Text "`r`n$($Summary.WorkloadName)`r`n"
  Add-Utf8Text -Path $NotesPath -Text ("-" * $Summary.WorkloadName.Length)
  Add-Utf8Text -Path $NotesPath -Text "`r`n"
  Add-Utf8Text -Path $NotesPath -Text "trials loaded: $($Summary.TrialCount)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "matched records: $($Summary.MatchedRecords)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "all agreement pass: $($Summary.AllAgreementPass.ToString().ToLowerInvariant())`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean fresh owned ns: $(Format-InvariantDouble -Value $Summary.MeanFreshOwnedElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reusable owned ns: $(Format-InvariantDouble -Value $Summary.MeanReusableOwnedElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean reference ns: $(Format-InvariantDouble -Value $Summary.MeanReferenceElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean visitor ns: $(Format-InvariantDouble -Value $Summary.MeanVisitorElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean row-view ns: $(Format-InvariantDouble -Value $Summary.MeanRowViewElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "mean count-only ns: $(Format-InvariantDouble -Value $Summary.MeanCountOnlyElapsedNs)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "row-view faster than reference trials: $($Summary.RowViewFasterThanReferenceTrials)`r`n"
  Add-Utf8Text -Path $NotesPath -Text "row-view faster than visitor trials: $($Summary.RowViewFasterThanVisitorTrials)`r`n"
}

Add-Utf8Text -Path $NotesPath -Text "`r`nDecision guidance`r`n"
Add-Utf8Text -Path $NotesPath -Text "-----------------`r`n"
Add-Utf8Text -Path $NotesPath -Text "Use this script when single-run materialization summaries are too noisy to judge output-contract movement.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Treat the detail CSV as raw repeated evidence and the summary CSV as the review convenience artifact.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Do not use row-view timing as evidence unless all_agreement_pass is true for the workload being discussed.`r`n"
Add-Utf8Text -Path $NotesPath -Text "Keep row-view conclusions separate from owned-result, reference-visitor, count-only, and existence conclusions.`r`n"

Write-Host ""
Write-Host "Materialization mode trial artifacts written:"
Write-Host "  Dataset: $Dataset"
Write-Host "  Max depth: $EffectiveMaxDepth"
Write-Host "  $TrialOutputDir"
Write-Host "  $TrialLogPath"
Write-Host "  $DetailCsvPath"
Write-Host "  $SummaryCsvPath"
Write-Host "  $NotesPath"
