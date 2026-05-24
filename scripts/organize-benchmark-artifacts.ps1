param(
  [string]$ArtifactRoot = "benchmark_artifacts",
  [string]$RunRoot = "",
  [string]$Label = "",
  [switch]$DryRun,
  [switch]$Copy,
  [switch]$Force
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ArtifactRoot)) {
  throw "`-ArtifactRoot` cannot be empty"
}

if (!(Test-Path -LiteralPath $ArtifactRoot)) {
  throw "artifact root was not found: $ArtifactRoot"
}

if ([string]::IsNullOrWhiteSpace($RunRoot)) {
  $RunRoot = Join-Path $ArtifactRoot "runs"
}

$ArtifactRootPath = (Resolve-Path -LiteralPath $ArtifactRoot).Path

$ReservedDirectoryNames = @(
  "runs",
  "archive"
)

$FilePrefixes = @(
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

$DirectoryPrefixes = @(
  "low-gap-trials-",
  "leaf-policy-trials-"
) | Sort-Object -Property Length -Descending

function Get-ArtifactLabelFromName {
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

function Get-ArtifactLabel {
  param(
    [System.IO.FileSystemInfo]$Item
  )

  if ($Item.PSIsContainer) {
    return Get-ArtifactLabelFromName -Name $Item.Name -Prefixes $DirectoryPrefixes
  }

  $NameWithoutExtension = [System.IO.Path]::GetFileNameWithoutExtension($Item.Name)

  return Get-ArtifactLabelFromName -Name $NameWithoutExtension -Prefixes $FilePrefixes
}

function Test-LabelMatches {
  param(
    [string]$DiscoveredLabel
  )

  if ([string]::IsNullOrWhiteSpace($Label)) {
    return $true
  }

  return $DiscoveredLabel -eq $Label
}

function New-RunDirectory {
  param(
    [string]$DiscoveredLabel
  )

  $DestinationDirectory = Join-Path $RunRoot $DiscoveredLabel

  if ($DryRun) {
    return $DestinationDirectory
  }

  New-Item -ItemType Directory -Force -Path $DestinationDirectory | Out-Null

  return $DestinationDirectory
}

function Move-OrCopy-Artifact {
  param(
    [System.IO.FileSystemInfo]$Item,
    [string]$DestinationPath
  )

  $Action = if ($Copy) { "copy" } else { "move" }

  if ($DryRun) {
    Write-Host "DRY RUN: $Action `"$($Item.FullName)`" -> `"$DestinationPath`""
    return
  }

  if ((Test-Path -LiteralPath $DestinationPath) -and !$Force) {
    Write-Host "skip existing: $DestinationPath"
    return
  }

  if ($Copy) {
    Copy-Item `
      -LiteralPath $Item.FullName `
      -Destination $DestinationPath `
      -Recurse `
      -Force:$Force
  }
  else {
    Move-Item `
      -LiteralPath $Item.FullName `
      -Destination $DestinationPath `
      -Force:$Force
  }

  Write-Host "$Action`: $($Item.Name) -> $DestinationPath"
}

$Items = Get-ChildItem -LiteralPath $ArtifactRootPath -Force

$RecognizedItems = New-Object System.Collections.Generic.List[object]
$UnrecognizedItems = New-Object System.Collections.Generic.List[object]

foreach ($Item in $Items) {
  if ($Item.PSIsContainer -and ($ReservedDirectoryNames -contains $Item.Name)) {
    continue
  }

  $DiscoveredLabel = Get-ArtifactLabel -Item $Item

  if ([string]::IsNullOrWhiteSpace($DiscoveredLabel)) {
    $UnrecognizedItems.Add($Item)
    continue
  }

  if (!(Test-LabelMatches -DiscoveredLabel $DiscoveredLabel)) {
    continue
  }

  $RecognizedItems.Add([PSCustomObject]@{
      Item  = $Item
      Label = $DiscoveredLabel
    })
}

if ($RecognizedItems.Count -eq 0) {
  if ([string]::IsNullOrWhiteSpace($Label)) {
    Write-Host "No recognized benchmark artifacts found in: $ArtifactRootPath"
  }
  else {
    Write-Host "No recognized benchmark artifacts found for label: $Label"
  }

  if ($UnrecognizedItems.Count -gt 0) {
    Write-Host "Unrecognized top-level items: $($UnrecognizedItems.Count)"
  }

  return
}

Write-Host "Benchmark artifact organization"
Write-Host "==============================="
Write-Host ""
Write-Host "Artifact root: $ArtifactRootPath"
Write-Host "Run root:      $RunRoot"
Write-Host "Mode:          $(if ($Copy) { "copy" } else { "move" })"
Write-Host "Dry run:       $DryRun"
Write-Host "Force:         $Force"

if (![string]::IsNullOrWhiteSpace($Label)) {
  Write-Host "Label filter:  $Label"
}

Write-Host ""

$GroupedItems = $RecognizedItems | Group-Object -Property Label | Sort-Object -Property Name

foreach ($Group in $GroupedItems) {
  $DiscoveredLabel = $Group.Name
  $DestinationDirectory = New-RunDirectory -DiscoveredLabel $DiscoveredLabel

  Write-Host "Label: $DiscoveredLabel"
  Write-Host "Destination: $DestinationDirectory"

  foreach ($Entry in ($Group.Group | Sort-Object { $_.Item.Name })) {
    $Item = $Entry.Item
    $DestinationPath = Join-Path $DestinationDirectory $Item.Name

    Move-OrCopy-Artifact -Item $Item -DestinationPath $DestinationPath
  }

  Write-Host ""
}

Write-Host "Organization summary"
Write-Host "--------------------"
Write-Host "Labels organized:       $($GroupedItems.Count)"
Write-Host "Artifacts organized:    $($RecognizedItems.Count)"
Write-Host "Unrecognized top-level: $($UnrecognizedItems.Count)"

if ($UnrecognizedItems.Count -gt 0) {
  Write-Host ""
  Write-Host "Unrecognized top-level items were left in place."

  foreach ($Item in ($UnrecognizedItems | Sort-Object -Property Name)) {
    Write-Host "  $($Item.Name)"
  }
}

if (!$DryRun -and !$Copy) {
  Write-Host ""
  Write-Host "Note: moved artifacts are no longer in the flat artifact root."
  Write-Host "Previous-label comparisons expect previous artifacts to be in the selected OutputDir."
  Write-Host "Use -Copy instead of moving if you want to preserve the flat comparison layout."
}