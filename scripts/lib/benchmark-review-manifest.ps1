$BenchmarkReviewManifestStateFound = "found"
$BenchmarkReviewManifestStateMissing = "missing"
$BenchmarkReviewManifestStateSkipped = "skipped"

$BenchmarkReviewArtifactNames = [ordered]@{
  BenchmarkOutput                    = "benchmark output"
  DebugOutput                        = "debug output"
  SummaryCsv                         = "summary CSV"
  WorkloadsCsv                       = "workloads CSV"
  CountOnlyWorkloadSummary           = "count-only workload summary"
  CountOnlyWorkloadNotes             = "count-only workload notes"
  CountOnlyWorkloadComparisonCsv     = "count-only workload comparison CSV"
  CountOnlyWorkloadComparisonNotes   = "count-only workload comparison notes"
  LowSelectivityGapCsv               = "low-selectivity gap CSV"
  LowGapRegressionNotes              = "low-gap regression notes"
  LowGapTrialDetails                 = "low-gap trial details"
  LowGapTrialSummary                 = "low-gap trial summary"
  LowGapTrialNotes                   = "low-gap trial notes"
  LowGapWorkloadTrialDetails         = "low-gap workload trial details"
  LowGapWorkloadTrialSummary         = "low-gap workload trial summary"
  AggregateTrialComparisonCsv        = "aggregate trial comparison CSV"
  AggregateTrialComparisonNotes      = "aggregate trial comparison notes"
  WorkloadTrialComparisonCsv         = "workload trial comparison CSV"
  WorkloadTrialComparisonNotes       = "workload trial comparison notes"
  TargetWorkloadTrialDetails         = "target workload trial details"
  TargetWorkloadTrialSummary         = "target workload trial summary"
  TargetWorkloadTrialNotes           = "target workload trial notes"
  TargetWorkloadTrialComparisonCsv   = "target workload trial comparison CSV"
  TargetWorkloadTrialComparisonNotes = "target workload trial comparison notes"
  LowGapReviewNotes                  = "low-gap review notes"
  LowGapReviewManifest               = "low-gap review manifest"
}


function Write-BenchmarkReviewManifestHeader {
  param(
    [string]$ManifestPath,
    [string]$Label,
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
    [string]$NoiseThresholdText
  )

  Set-Utf8Text -Path $ManifestPath -Text "Low-gap review artifact manifest`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "================================`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "`r`n"
  Add-Utf8Text -Path $ManifestPath -Text "Label: $Label`r`n"
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


function Get-BenchmarkReviewArtifactState {
  param(
    [string]$Path,
    [bool]$Skipped
  )

  if ($Skipped) {
    return $BenchmarkReviewManifestStateSkipped
  }

  if (Test-Path -LiteralPath $Path) {
    return $BenchmarkReviewManifestStateFound
  }

  return $BenchmarkReviewManifestStateMissing
}

function Add-BenchmarkReviewManifestArtifactLine {
  param(
    [string]$ManifestPath,
    [string]$Name,
    [string]$Path,
    [bool]$Skipped,
    [string]$Note = ""
  )

  $State = Get-BenchmarkReviewArtifactState -Path $Path -Skipped $Skipped

  if ([string]::IsNullOrWhiteSpace($Note)) {
    Add-Utf8Text -Path $ManifestPath -Text "$State | $Name | $Path`r`n"
  }
  else {
    Add-Utf8Text -Path $ManifestPath -Text "$State | $Name | $Path | $Note`r`n"
  }
}


function New-BenchmarkReviewManifestArtifact {
  param(
    [string]$Name,
    [string]$Path,
    [bool]$Skipped,
    [string]$Note = ""
  )

  return [PSCustomObject]@{
    Name    = $Name
    Path    = $Path
    Skipped = $Skipped
    Note    = $Note
  }
}

function Add-BenchmarkReviewManifestArtifacts {
  param(
    [string]$ManifestPath,
    [object[]]$Artifacts
  )

  foreach ($Artifact in $Artifacts) {
    Add-BenchmarkReviewManifestArtifactLine `
      -ManifestPath $ManifestPath `
      -Name $Artifact.Name `
      -Path $Artifact.Path `
      -Skipped ([bool]$Artifact.Skipped) `
      -Note $Artifact.Note
  }
}

function Test-BenchmarkReviewManifestArtifactLine {
  param(
    [string]$Line
  )

  return (
    $Line.StartsWith("$BenchmarkReviewManifestStateFound |") -or
    $Line.StartsWith("$BenchmarkReviewManifestStateMissing |") -or
    $Line.StartsWith("$BenchmarkReviewManifestStateSkipped |")
  )
}

function ConvertTo-BenchmarkReviewManifestEntry {
  param(
    [string]$Line,
    [scriptblock]$OnMalformedArtifactLine = $null
  )

  $TrimmedLine = $Line.Trim()

  if (!(Test-BenchmarkReviewManifestArtifactLine -Line $TrimmedLine)) {
    return $null
  }

  $Parts = @(
    $Line -split "\|", 4 |
    ForEach-Object { $_.Trim() }
  )

  if ($Parts.Count -lt 3) {
    if ($null -ne $OnMalformedArtifactLine) {
      & $OnMalformedArtifactLine $Line
    }

    return $null
  }

  $Note = ""

  if ($Parts.Count -ge 4) {
    $Note = $Parts[3]
  }

  return [PSCustomObject]@{
    State    = $Parts[0]
    Artifact = $Parts[1]
    Path     = $Parts[2]
    Note     = $Note
  }
}

function Read-BenchmarkReviewManifestEntries {
  param(
    [string]$Path,
    [scriptblock]$OnMalformedArtifactLine = $null
  )

  $Entries = @()

  if (!(Test-Path -LiteralPath $Path)) {
    return $Entries
  }

  $Lines = Get-Content -LiteralPath $Path

  foreach ($Line in $Lines) {
    $Entry = ConvertTo-BenchmarkReviewManifestEntry `
      -Line $Line `
      -OnMalformedArtifactLine $OnMalformedArtifactLine

    if ($null -ne $Entry) {
      $Entries += $Entry
    }
  }

  return $Entries
}

function Get-BenchmarkReviewManifestEntry {
  param(
    [object[]]$Entries,
    [string]$ArtifactName
  )

  $Matches = @(
    $Entries |
    Where-Object { $_.Artifact -eq $ArtifactName }
  )

  if ($Matches.Count -eq 0) {
    return $null
  }

  return $Matches[0]
}
function Test-BenchmarkReviewPathExists {
  param(
    [string]$Path,
    [scriptblock]$OnInvalidPath = $null
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $false
  }

  try {
    return Test-Path -LiteralPath $Path
  }
  catch {
    if ($null -ne $OnInvalidPath) {
      & $OnInvalidPath $Path
    }

    return $false
  }
}

function Resolve-BenchmarkReviewArtifactPath {
  param(
    [string]$Path,
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnInvalidPath = $null
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $Path
  }

  if (
    Test-BenchmarkReviewPathExists `
      -Path $Path `
      -OnInvalidPath $OnInvalidPath
  ) {
    return (Resolve-Path -LiteralPath $Path).Path
  }

  $FileName = Split-Path -Leaf $Path

  if ([string]::IsNullOrWhiteSpace($FileName)) {
    return $Path
  }

  $OrganizedRunDirectory = Join-Path (Join-Path (Join-Path $OutputDir "runs") $Label) ""
  $OrganizedPath = Join-Path $OrganizedRunDirectory $FileName

  if (
    Test-BenchmarkReviewPathExists `
      -Path $OrganizedPath `
      -OnInvalidPath $OnInvalidPath
  ) {
    return (Resolve-Path -LiteralPath $OrganizedPath).Path
  }

  return $Path
}

function Get-BenchmarkReviewManifestPath {
  param(
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnInvalidPath = $null
  )

  $FlatManifestPath = Join-Path $OutputDir "low-gap-review-manifest-$Label.txt"
  $ResolvedFlatManifestPath = Resolve-BenchmarkReviewArtifactPath `
    -Path $FlatManifestPath `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath $OnInvalidPath

  if (
    Test-BenchmarkReviewPathExists `
      -Path $ResolvedFlatManifestPath `
      -OnInvalidPath $OnInvalidPath
  ) {
    return (Resolve-Path -LiteralPath $ResolvedFlatManifestPath).Path
  }

  return $FlatManifestPath
}


function Resolve-BenchmarkReviewManifestArtifact {
  param(
    [object[]]$Entries,
    [string]$ArtifactName,
    [ValidateSet("required", "comparison", "target")]
    [string]$Kind = "required",
    [bool]$RequireComparisons = $false,
    [bool]$AllowSkippedTargetWorkloadReview = $false,
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnFailure,
    [scriptblock]$OnInvalidPath = $null
  )

  $Entry = Get-BenchmarkReviewManifestEntry -Entries $Entries -ArtifactName $ArtifactName

  if ($null -eq $Entry) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest does not list required artifact: $ArtifactName"
    }

    return $null
  }

  if ($Entry.State -eq $BenchmarkReviewManifestStateMissing) {
    if ($Kind -eq "comparison" -and !$RequireComparisons) {
      return $Entry
    }

    if ($null -ne $OnFailure) {
      & $OnFailure "manifest reports missing artifact '$ArtifactName': $($Entry.Path)"
    }

    return $Entry
  }

  if ($Entry.State -eq $BenchmarkReviewManifestStateSkipped) {
    if ($Kind -eq "comparison" -and !$RequireComparisons) {
      return $Entry
    }

    if ($Kind -eq "target" -and $AllowSkippedTargetWorkloadReview) {
      return $Entry
    }

    if ($null -ne $OnFailure) {
      & $OnFailure "manifest reports skipped artifact '$ArtifactName': $($Entry.Note)"
    }

    return $Entry
  }

  if ($Entry.State -ne $BenchmarkReviewManifestStateFound) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest has unknown state '$($Entry.State)' for artifact '$ArtifactName'"
    }

    return $Entry
  }

  $ResolvedPath = Resolve-BenchmarkReviewArtifactPath `
    -Path $Entry.Path `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath $OnInvalidPath

  if (
    !(
      Test-BenchmarkReviewPathExists `
        -Path $ResolvedPath `
        -OnInvalidPath $OnInvalidPath
    )
  ) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest reports found artifact '$ArtifactName' but file was not found: $($Entry.Path)"
    }

    return $Entry
  }

  $Entry.Path = $ResolvedPath

  return $Entry
}


function Test-BenchmarkReviewNonEmptyFile {
  param(
    [object]$Entry,
    [string]$Description,
    [scriptblock]$OnFailure
  )

  if ($null -eq $Entry) {
    return
  }

  if ($Entry.State -ne $BenchmarkReviewManifestStateFound) {
    return
  }

  if (!(Test-BenchmarkReviewPathExists -Path $Entry.Path)) {
    & $OnFailure "$Description was not found: $($Entry.Path)"
    return
  }

  $Item = Get-Item -LiteralPath $Entry.Path

  if ($Item.Length -le 0) {
    & $OnFailure "$Description is empty: $($Entry.Path)"
  }
}

function Read-BenchmarkReviewValidatedCsv {
  param(
    [object]$Entry,
    [string]$Description,
    [string[]]$RequiredColumns,
    [int]$MinimumRows = 1,
    [scriptblock]$OnFailure
  )

  if ($null -eq $Entry) {
    return @()
  }

  if ($Entry.State -ne $BenchmarkReviewManifestStateFound) {
    return @()
  }

  if (!(Test-BenchmarkReviewPathExists -Path $Entry.Path)) {
    & $OnFailure "$Description was not found: $($Entry.Path)"
    return @()
  }

  try {
    $Rows = @(Import-Csv -LiteralPath $Entry.Path)
  }
  catch {
    & $OnFailure "$Description could not be parsed as CSV: $($Entry.Path)"
    & $OnFailure "  $($_.Exception.Message)"
    return @()
  }

  if ($Rows.Count -lt $MinimumRows) {
    & $OnFailure "$Description has fewer rows than expected. expected at least $MinimumRows, found $($Rows.Count): $($Entry.Path)"
    return $Rows
  }

  $Columns = @(
    $Rows[0].PSObject.Properties |
    ForEach-Object { $_.Name }
  )

  foreach ($ColumnName in $RequiredColumns) {
    if ($Columns -notcontains $ColumnName) {
      & $OnFailure "$Description is missing required column '$ColumnName': $($Entry.Path)"
    }
  }

  return $Rows
}

function Test-BenchmarkReviewBooleanColumnAllTrue {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string]$Description,
    [scriptblock]$OnFailure
  )

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue) {
      & $OnFailure "$Description has missing value in column '$ColumnName'"
      continue
    }

    $TextValue = $RawValue.ToString().Trim().ToLowerInvariant()

    if ($TextValue -ne "true") {
      & $OnFailure "$Description has non-true value in column '$ColumnName': $RawValue"
    }
  }
}

function Test-BenchmarkReviewRequiredBaselines {
  param(
    [object[]]$Rows,
    [string]$Description,
    [scriptblock]$OnFailure
  )

  $BaselineNames = @(
    $Rows |
    ForEach-Object { $_.baseline_name } |
    Where-Object { ![string]::IsNullOrWhiteSpace($_) } |
    Sort-Object -Unique
  )

  foreach ($BaselineName in @("kd_tree", "r_tree")) {
    if ($BaselineNames -notcontains $BaselineName) {
      & $OnFailure "$Description does not contain required baseline row: $BaselineName"
    }
  }
}

function Test-BenchmarkReviewExpectedTrialCount {
  param(
    [object[]]$Rows,
    [string]$ColumnName,
    [string]$Description,
    [int]$ExpectedTrials,
    [scriptblock]$OnFailure
  )

  if ($ExpectedTrials -le 0) {
    return
  }

  foreach ($Row in $Rows) {
    $RawValue = $Row.$ColumnName

    if ($null -eq $RawValue -or [string]::IsNullOrWhiteSpace($RawValue.ToString())) {
      & $OnFailure "$Description has missing trial count in column '$ColumnName'"
      continue
    }

    try {
      $TrialCount = [int][double]::Parse(
        $RawValue.ToString(),
        [System.Globalization.CultureInfo]::InvariantCulture
      )

      if ($TrialCount -ne $ExpectedTrials) {
        & $OnFailure "$Description expected trial count $ExpectedTrials but found $TrialCount"
      }
    }
    catch {
      & $OnFailure "$Description has invalid trial count value '$RawValue'"
    }
  }
}

function Read-BenchmarkReviewPolicyValidatedCsv {
  param(
    [object]$Entry,
    [string]$Description,
    [string[]]$RequiredColumns,
    [int]$MinimumRows = 1,
    [string[]]$BooleanTrueColumns = @(),
    [bool]$RequireBaselines = $false,
    [string]$TrialCountColumn = "",
    [int]$ExpectedTrials = 0,
    [scriptblock]$OnFailure
  )

  $Rows = Read-BenchmarkReviewValidatedCsv `
    -Entry $Entry `
    -Description $Description `
    -RequiredColumns $RequiredColumns `
    -MinimumRows $MinimumRows `
    -OnFailure $OnFailure

  foreach ($ColumnName in $BooleanTrueColumns) {
    Test-BenchmarkReviewBooleanColumnAllTrue `
      -Rows $Rows `
      -ColumnName $ColumnName `
      -Description $Description `
      -OnFailure $OnFailure
  }

  if ($RequireBaselines) {
    Test-BenchmarkReviewRequiredBaselines `
      -Rows $Rows `
      -Description $Description `
      -OnFailure $OnFailure
  }

  if (![string]::IsNullOrWhiteSpace($TrialCountColumn)) {
    Test-BenchmarkReviewExpectedTrialCount `
      -Rows $Rows `
      -ColumnName $TrialCountColumn `
      -Description $Description `
      -ExpectedTrials $ExpectedTrials `
      -OnFailure $OnFailure
  }

  return $Rows
}

