$BenchmarkArtifactReservedDirectoryNames = @(
  "runs",
  "archive"
)

$BenchmarkArtifactFilePrefixes = @(
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
