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

function Split-BenchmarkHistoryHeader {
  param(
    [string]$HeaderLine
  )

  if ([string]::IsNullOrWhiteSpace($HeaderLine)) {
    return @()
  }

  return @($HeaderLine.Split(","))
}

function Test-BenchmarkHistoryHeaderCanExpand {
  param(
    [string[]]$ExistingHeader,
    [string[]]$ExpectedHeader
  )

  $ExpectedColumns = @{}

  foreach ($Column in $ExpectedHeader) {
    $ExpectedColumns[$Column] = $true
  }

  foreach ($Column in $ExistingHeader) {
    if (!$ExpectedColumns.ContainsKey($Column)) {
      return $false
    }
  }

  return $true
}

function Update-BenchmarkHistoryHeader {
  param(
    [string]$Path,
    [string[]]$Header
  )

  $ExistingRows = @(Import-Csv -LiteralPath $Path)

  Set-Utf8Text -Path $Path -Text "$(Join-CsvFields -Fields $Header)`r`n"

  foreach ($Row in $ExistingRows) {
    $Values = @(
      foreach ($Column in $Header) {
        Get-BenchmarkHistoryRowValue -Row $Row -ColumnName $Column
      }
    )

    Add-Utf8Text -Path $Path -Text "$(Join-CsvFields -Fields $Values)`r`n"
  }
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
    $ExistingHeaderFields = Split-BenchmarkHistoryHeader -HeaderLine $ExistingHeader

    if (Test-BenchmarkHistoryHeaderCanExpand -ExistingHeader $ExistingHeaderFields -ExpectedHeader $Header) {
      Update-BenchmarkHistoryHeader -Path $Path -Header $Header
      return
    }

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

function Get-BenchmarkFootprintHistoryColumns {
  return @(
    "dataset_records",
    "index_nodes",
    "run_workload_count",
    "selected_baselines",
    "timing_iterations",
    "target_leaf_size",
    "max_leaf_size",
    "max_depth",
    "fse_execution_mode",
    "fse_parallel_min_retained_leaves",
    "index_leaf_count",
    "index_internal_node_count",
    "index_total_leaf_cardinality",
    "index_min_leaf_cardinality",
    "index_max_leaf_cardinality",
    "index_average_leaf_cardinality",
    "index_total_leaf_volume",
    "index_average_leaf_volume",
    "index_density",
    "index_zero_volume_leaf_count",
    "index_encoded_coordinate_scalar_count",
    "index_residual_scalar_count",
    "index_centroid_scalar_count",
    "index_bounds_scalar_count",
    "index_structural_metadata_scalar_count",
    "index_total_scalar_count",
    "index_residual_to_encoded_scalar_ratio",
    "index_structural_to_encoded_scalar_ratio",
    "index_to_encoded_scalar_ratio",
    "index_valid",
    "node_identifier_consistency_valid",
    "partition_dimensional_metadata_valid",
    "leaf_cardinality_valid",
    "leaf_reconstruction_metadata_valid",
    "leaf_record_bounds_valid",
    "leaf_ownership_cardinality_valid",
    "hierarchy_topology_valid",
    "parent_child_bounds_valid"
  )
}

function Assert-BenchmarkHistoryRequiredColumns {
  param(
    [object]$Row,
    [string[]]$Columns,
    [string]$SourceCsv
  )

  foreach ($Column in $Columns) {
    if ($null -eq $Row.PSObject.Properties[$Column]) {
      throw "benchmark history source CSV is missing required column '$Column': $SourceCsv"
    }
  }
}

function Add-BenchmarkFootprintSummaryToHistory {
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

  $SourceColumns = Get-BenchmarkFootprintHistoryColumns
  $Metadata = New-BenchmarkHistoryMetadata -Context $Context -SourceCsv $ResolvedSourceCsv
  $MetadataColumns = @($Metadata.Keys)
  $Header = @($MetadataColumns + $SourceColumns)
  $SeenRows = @{}

  $HistoryRows = @(
    foreach ($SourceRow in $SourceRows) {
      Assert-BenchmarkHistoryRequiredColumns `
        -Row $SourceRow `
        -Columns $SourceColumns `
        -SourceCsv $ResolvedSourceCsv

      $SourceValues = @(
        foreach ($Column in $SourceColumns) {
          Get-BenchmarkHistoryRowValue -Row $SourceRow -ColumnName $Column
        }
      )
      $SourceKey = Join-CsvFields -Fields $SourceValues

      if ($SeenRows.ContainsKey($SourceKey)) {
        continue
      }

      $SeenRows[$SourceKey] = $true
      $HistoryRow = [ordered]@{}

      foreach ($Column in $MetadataColumns) {
        $HistoryRow[$Column] = $Metadata[$Column]
      }

      foreach ($Index in 0..($SourceColumns.Count - 1)) {
        $HistoryRow[$SourceColumns[$Index]] = $SourceValues[$Index]
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

  Add-BenchmarkFootprintSummaryToHistory `
    -Context $Context `
    -SourceCsv (Join-Path $Context.OutputDir "summary-$($Context.Label).csv") `
    -HistoryCsv (Join-Path $Context.HistoryDir "index-footprint-history.csv")
}
