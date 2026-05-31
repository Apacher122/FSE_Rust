function Get-BenchmarkHistorySourceColumns {
  param(
    [object[]]$Rows
  )

  if ($Rows.Count -eq 0) {
    return @()
  }

  return @($Rows[0].PSObject.Properties | ForEach-Object { $_.Name })
}

function Get-BenchmarkHistoryRowValue {
  param(
    [object]$Row,
    [string]$ColumnName
  )

  $Property = $Row.PSObject.Properties[$ColumnName]

  if ($null -eq $Property) {
    return ""
  }

  return $Property.Value
}

function Assert-BenchmarkHistoryHeader {
  param(
    [string]$Path,
    [string[]]$Header
  )

  $ExpectedHeader = Join-CsvFields -Fields $Header

  if (!(Test-Path -LiteralPath $Path)) {
    Add-Utf8Text -Path $Path -Text "$ExpectedHeader`r`n"
    return
  }

  $ExistingItem = Get-Item -LiteralPath $Path

  if ($ExistingItem.Length -eq 0) {
    Add-Utf8Text -Path $Path -Text "$ExpectedHeader`r`n"
    return
  }

  $ExistingLines = [System.IO.File]::ReadAllLines($Path)

  if ($ExistingLines.Count -eq 0) {
    Add-Utf8Text -Path $Path -Text "$ExpectedHeader`r`n"
    return
  }

  $ExistingHeader = $ExistingLines[0]

  if ($ExistingHeader -ne $ExpectedHeader) {
    throw "benchmark history header mismatch for $Path"
  }
}

function Add-BenchmarkHistoryRows {
  param(
    [string]$Path,
    [string[]]$Header,
    [object[]]$Rows
  )

  if ($Rows.Count -eq 0) {
    return
  }

  $ParentDirectory = Split-Path -Parent $Path

  if (![string]::IsNullOrWhiteSpace($ParentDirectory)) {
    New-Item -ItemType Directory -Force -Path $ParentDirectory | Out-Null
  }

  Assert-BenchmarkHistoryHeader -Path $Path -Header $Header

  foreach ($Row in $Rows) {
    $Values = @(
      foreach ($Column in $Header) {
        Get-BenchmarkHistoryRowValue -Row $Row -ColumnName $Column
      }
    )

    Add-Utf8Text -Path $Path -Text "$(Join-CsvFields -Fields $Values)`r`n"
  }
}

function New-BenchmarkHistoryMetadata {
  param(
    [object]$Context,
    [string]$SourceCsv
  )

  return [ordered]@{
    run_id              = $Context.RunId
    attempt_id          = $Context.AttemptId
    run_label           = $Context.Label
    run_topic           = $Context.RunTopic
    run_dataset         = $Context.Dataset
    run_max_depth       = $Context.EffectiveMaxDepth
    run_trials          = $Context.Trials
    run_iterations      = $Context.Iterations
    run_target_workload = $Context.TargetWorkloadName
    source_csv          = $SourceCsv
  }
}

function Resolve-BenchmarkHistorySourceCsv {
  param(
    [object]$Context,
    [string]$SourceCsv
  )

  if ([string]::IsNullOrWhiteSpace($SourceCsv)) {
    return ""
  }

  $SourceFileName = Split-Path -Leaf $SourceCsv

  if (![string]::IsNullOrWhiteSpace($Context.OrganizedRunDirectory)) {
    $OrganizedSourceCsv = Join-Path $Context.OrganizedRunDirectory $SourceFileName

    if (Test-Path -LiteralPath $OrganizedSourceCsv) {
      return $OrganizedSourceCsv
    }
  }

  return $SourceCsv
}

function Add-BenchmarkSourceCsvToHistory {
  param(
    [object]$Context,
    [string]$SourceCsv,
    [string]$HistoryCsv
  )

  if ([string]::IsNullOrWhiteSpace($SourceCsv)) {
    return
  }

  $ResolvedSourceCsv = Resolve-BenchmarkHistorySourceCsv `
    -Context $Context `
    -SourceCsv $SourceCsv

  if (!(Test-Path -LiteralPath $ResolvedSourceCsv)) {
    return
  }

  $SourceRows = @(Import-Csv -LiteralPath $ResolvedSourceCsv)

  if ($SourceRows.Count -eq 0) {
    return
  }

  $SourceColumns = Get-BenchmarkHistorySourceColumns -Rows $SourceRows
  $Metadata = New-BenchmarkHistoryMetadata -Context $Context -SourceCsv $ResolvedSourceCsv
  $MetadataColumns = @($Metadata.Keys)
  $Header = @($MetadataColumns + $SourceColumns)

  $HistoryRows = @(
    foreach ($SourceRow in $SourceRows) {
      $HistoryRow = [ordered]@{}

      foreach ($Column in $MetadataColumns) {
        $HistoryRow[$Column] = $Metadata[$Column]
      }

      foreach ($Column in $SourceColumns) {
        $HistoryRow[$Column] = Get-BenchmarkHistoryRowValue -Row $SourceRow -ColumnName $Column
      }

      [PSCustomObject]$HistoryRow
    }
  )

  Add-BenchmarkHistoryRows -Path $HistoryCsv -Header $Header -Rows $HistoryRows
}

function Add-BenchmarkRunToHistory {
  param(
    [object]$Context
  )

  $HistoryCsv = Join-Path $Context.HistoryDir "benchmark-run-history.csv"

  $Header = @(
    "run_id",
    "attempt_id",
    "run_label",
    "run_topic",
    "run_dataset",
    "run_max_depth",
    "run_trials",
    "run_iterations",
    "run_target_workload",
    "flat_artifact_directory",
    "organized_artifact_directory",
    "review_notes_path",
    "review_manifest_path"
  )

  $Row = [PSCustomObject]@{
    run_id                       = $Context.RunId
    attempt_id                   = $Context.AttemptId
    run_label                    = $Context.Label
    run_topic                    = $Context.RunTopic
    run_dataset                  = $Context.Dataset
    run_max_depth                = $Context.EffectiveMaxDepth
    run_trials                   = $Context.Trials
    run_iterations               = $Context.Iterations
    run_target_workload          = $Context.TargetWorkloadName
    flat_artifact_directory      = $Context.OutputDir
    organized_artifact_directory = $Context.OrganizedRunDirectory
    review_notes_path            = $Context.ReviewNotesPath
    review_manifest_path         = $Context.ReviewManifestPath
  }

  Add-BenchmarkHistoryRows -Path $HistoryCsv -Header $Header -Rows @($Row)
}

function Update-BenchmarkReviewHistory {
  param(
    [object]$Context
  )

  New-Item -ItemType Directory -Force -Path $Context.HistoryDir | Out-Null

  Add-BenchmarkRunToHistory -Context $Context

  Add-BenchmarkSourceCsvToHistory `
    -Context $Context `
    -SourceCsv $Context.CurrentTrialSummaryCsv `
    -HistoryCsv (Join-Path $Context.HistoryDir "low-selectivity-performance-history.csv")

  Add-BenchmarkSourceCsvToHistory `
    -Context $Context `
    -SourceCsv $Context.CurrentWorkloadSummaryCsv `
    -HistoryCsv (Join-Path $Context.HistoryDir "weakest-workload-history.csv")

  Add-BenchmarkSourceCsvToHistory `
    -Context $Context `
    -SourceCsv $Context.CurrentCountOnlyWorkloadSummaryCsv `
    -HistoryCsv (Join-Path $Context.HistoryDir "count-only-workload-history.csv")

  Add-BenchmarkSourceCsvToHistory `
    -Context $Context `
    -SourceCsv $Context.CurrentMaterializationSummaryCsv `
    -HistoryCsv (Join-Path $Context.HistoryDir "materialization-mode-history.csv")

  Add-BenchmarkSourceCsvToHistory `
    -Context $Context `
    -SourceCsv $Context.CurrentTargetSummaryCsv `
    -HistoryCsv (Join-Path $Context.HistoryDir "target-workload-history.csv")
}
