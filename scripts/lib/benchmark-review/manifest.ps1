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

  $MatchingEntries = @(
    $Entries |
    Where-Object { $_.Artifact -eq $ArtifactName }
  )

  if ($MatchingEntries.Count -eq 0) {
    return $null
  }

  return $MatchingEntries[0]
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

function Get-BenchmarkReviewManifestLoadResult {
  param(
    [string]$OutputDir,
    [string]$Label,
    [bool]$RequireComparisons = $false,
    [bool]$AllowSkippedTargetWorkloadReview = $false,
    [scriptblock]$OnFailure,
    [scriptblock]$OnInvalidPath = $null
  )

  $ManifestPath = Get-BenchmarkReviewManifestPath `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnInvalidPath $OnInvalidPath

  if (
    !(
      Test-BenchmarkReviewPathExists `
        -Path $ManifestPath `
        -OnInvalidPath $OnInvalidPath
    )
  ) {
    if ($null -ne $OnFailure) {
      & $OnFailure "low-gap review manifest was not found: $ManifestPath"
    }

    return [pscustomobject]@{
      ManifestPath        = $ManifestPath
      Entries             = @()
      ResolvedArtifacts   = $null
      HasBlockingFailures = $true
    }
  }

  $MalformedArtifactLines = [System.Collections.Generic.List[string]]::new()

  $ManifestEntries = @(
    Read-BenchmarkReviewManifestEntries `
      -Path $ManifestPath `
      -OnMalformedArtifactLine {
      param($Line)

      $MalformedArtifactLines.Add($Line) | Out-Null

      if ($null -ne $OnFailure) {
        & $OnFailure "manifest has malformed artifact line: $Line"
      }
    }
  )

  if ($MalformedArtifactLines.Count -gt 0) {
    return [pscustomobject]@{
      ManifestPath        = $ManifestPath
      Entries             = $ManifestEntries
      ResolvedArtifacts   = $null
      HasBlockingFailures = $true
    }
  }

  if ($ManifestEntries.Count -eq 0) {
    if ($null -ne $OnFailure) {
      & $OnFailure "manifest did not contain any artifact entries: $ManifestPath"
    }

    return [pscustomobject]@{
      ManifestPath        = $ManifestPath
      Entries             = $ManifestEntries
      ResolvedArtifacts   = $null
      HasBlockingFailures = $true
    }
  }

  $ResolvedArtifacts = Resolve-BenchmarkReviewManifestArtifactSet `
    -Entries $ManifestEntries `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  return [pscustomobject]@{
    ManifestPath        = $ManifestPath
    Entries             = $ManifestEntries
    ResolvedArtifacts   = $ResolvedArtifacts
    HasBlockingFailures = $false
  }
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

function Resolve-BenchmarkReviewManifestArtifactSet {
  param(
    [object[]]$Entries,
    [bool]$RequireComparisons = $false,
    [bool]$AllowSkippedTargetWorkloadReview = $false,
    [string]$OutputDir,
    [string]$Label,
    [scriptblock]$OnFailure,
    [scriptblock]$OnInvalidPath = $null
  )

  $ResolvedArtifacts = [ordered]@{}

  $ResolvedArtifacts.BenchmarkOutput = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.BenchmarkOutput `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.DebugOutput = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.DebugOutput `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.Summary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.SummaryCsv `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.Workloads = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.WorkloadsCsv `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.CountOnlySummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.CountOnlyNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.MaterializationSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.MaterializationModeSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.MaterializationNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.MaterializationModeNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TypedArchiveLoadSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TypedArchiveLoadSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TypedArchiveLoadNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TypedArchiveLoadNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TypedArchiveAppendRebuildSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TypedArchiveAppendRebuildSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TypedArchiveAppendRebuildNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TypedArchiveAppendRebuildNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TypedQueryIndexArchive = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TypedQueryIndexArchive `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.LowGap = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowSelectivityGapCsv `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.RegressionNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapRegressionNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TrialDetails = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialDetails `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TrialSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TrialNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapTrialNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.WorkloadDetails = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapWorkloadTrialDetails `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.WorkloadSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapWorkloadTrialSummary `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetDetails = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialDetails `
    -Kind "target" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetSummary = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialSummary `
    -Kind "target" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialNotes `
    -Kind "target" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.ReviewNotes = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapReviewNotes `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.ReviewManifest = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.LowGapReviewManifest `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.AggregateComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.AggregateTrialComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.WorkloadComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.WorkloadTrialComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.CountOnlyComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.MaterializationComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.MaterializationModeComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  $ResolvedArtifacts.TargetComparison = Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonCsv `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.AggregateTrialComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.WorkloadTrialComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.CountOnlyWorkloadComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.MaterializationModeComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  Resolve-BenchmarkReviewManifestArtifact `
    -Entries $Entries `
    -ArtifactName $BenchmarkReviewArtifactNames.TargetWorkloadTrialComparisonNotes `
    -Kind "comparison" `
    -RequireComparisons $RequireComparisons `
    -AllowSkippedTargetWorkloadReview $AllowSkippedTargetWorkloadReview `
    -OutputDir $OutputDir `
    -Label $Label `
    -OnFailure $OnFailure `
    -OnInvalidPath $OnInvalidPath | Out-Null

  return [PSCustomObject]$ResolvedArtifacts
}
