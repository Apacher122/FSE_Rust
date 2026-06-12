$BenchmarkArtifactReservedDirectoryNames = @(
  "runs",
  "archive"
)

$BenchmarkArtifactFilePrefixes = @(
  "materialization-mode-summary-comparison-",
  "materialization-mode-trial-details-",
  "materialization-mode-trial-summary-",
  "materialization-mode-trial-notes-",
  "typed-query-index-archive-",
  "typed-archive-append-rebuild-summary-",
  "typed-indexed-comparison-summary-",
  "typed-archive-load-summary-",
  "existence-timing-summary-",
  "materialization-mode-summary-",
  "count-only-workload-summary-comparison-",
  "count-only-workload-summary-",
  "target-workload-trial-comparison-",
  "target-workload-trial-details-",
  "target-workload-trial-summary-",
  "target-workload-trial-notes-",
  "low-gap-workload-trial-comparison-",
  "low-gap-workload-trial-details-",
  "low-gap-workload-trial-summary-",
  "low-gap-trial-comparison-",
  "low-gap-trial-details-",
  "low-gap-trial-summary-",
  "low-gap-trial-notes-",
  "low-gap-review-manifest-",
  "low-gap-review-notes-",
  "low-gap-regression-notes-",
  "low-selectivity-gap-",
  "benchmark-output-",
  "debug-output-",
  "summary-",
  "workloads-"
) | Sort-Object -Property Length -Descending

$BenchmarkArtifactDirectoryPrefixes = @(
  "materialization-mode-trials-",
  "low-gap-trials-",
  "leaf-policy-trials-"
) | Sort-Object -Property Length -Descending

function Get-BenchmarkArtifactLabelFromName {
  param(
    [string]$Name,
    [string[]]$Prefixes
  )

  foreach ($Prefix in $Prefixes) {
    if ($Name.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
      return $Name.Substring($Prefix.Length)
    }
  }

  return ""
}

function Get-BenchmarkArtifactLabel {
  param(
    [System.IO.FileSystemInfo]$Item,
    [string[]]$FilePrefixes = $BenchmarkArtifactFilePrefixes,
    [string[]]$DirectoryPrefixes = $BenchmarkArtifactDirectoryPrefixes
  )

  if ($Item.PSIsContainer) {
    return Get-BenchmarkArtifactLabelFromName -Name $Item.Name -Prefixes $DirectoryPrefixes
  }

  $NameWithoutExtension = [System.IO.Path]::GetFileNameWithoutExtension($Item.Name)

  return Get-BenchmarkArtifactLabelFromName -Name $NameWithoutExtension -Prefixes $FilePrefixes
}

function Test-BenchmarkArtifactLabelMatches {
  param(
    [string]$DiscoveredLabel,
    [string]$ExpectedLabel
  )

  if ([string]::IsNullOrWhiteSpace($ExpectedLabel)) {
    return $true
  }

  return $DiscoveredLabel -eq $ExpectedLabel
}

function Find-BenchmarkArtifacts {
  param(
    [string]$ArtifactRootPath,
    [string]$ExpectedLabel = "",
    [string[]]$ReservedDirectoryNames = $BenchmarkArtifactReservedDirectoryNames,
    [string[]]$FilePrefixes = $BenchmarkArtifactFilePrefixes,
    [string[]]$DirectoryPrefixes = $BenchmarkArtifactDirectoryPrefixes
  )

  $RecognizedItems = New-Object System.Collections.Generic.List[object]
  $UnrecognizedItems = New-Object System.Collections.Generic.List[object]

  foreach ($Item in (Get-ChildItem -LiteralPath $ArtifactRootPath -Force)) {
    if ($Item.PSIsContainer -and ($ReservedDirectoryNames -contains $Item.Name)) {
      continue
    }

    $DiscoveredLabel = Get-BenchmarkArtifactLabel `
      -Item $Item `
      -FilePrefixes $FilePrefixes `
      -DirectoryPrefixes $DirectoryPrefixes

    if ([string]::IsNullOrWhiteSpace($DiscoveredLabel)) {
      $UnrecognizedItems.Add($Item)
      continue
    }

    if (!(Test-BenchmarkArtifactLabelMatches `
          -DiscoveredLabel $DiscoveredLabel `
          -ExpectedLabel $ExpectedLabel)) {
      continue
    }

    $RecognizedItems.Add([PSCustomObject]@{
        Item  = $Item
        Label = $DiscoveredLabel
      })
  }

  return [PSCustomObject]@{
    RecognizedItems   = $RecognizedItems
    UnrecognizedItems = $UnrecognizedItems
  }
}

