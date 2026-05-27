param(
  [string]$ArtifactRoot = "benchmark_artifacts",
  [string]$RunRoot = "",
  [string]$Label = "",
  [switch]$DryRun,
  [switch]$Copy,
  [switch]$Force
)

$ErrorActionPreference = "Stop"

$BenchmarkArtifactLibrary = Join-Path (Join-Path $PSScriptRoot "lib") "benchmark-artifacts.ps1"
. $BenchmarkArtifactLibrary

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
  if ($Item.PSIsContainer -and ($BenchmarkArtifactReservedDirectoryNames -contains $Item.Name)) {
    continue
  }

  $DiscoveredLabel = Get-BenchmarkArtifactLabel -Item $Item

  if ([string]::IsNullOrWhiteSpace($DiscoveredLabel)) {
    $UnrecognizedItems.Add($Item)
    continue
  }

  if (!(Test-BenchmarkArtifactLabelMatches -DiscoveredLabel $DiscoveredLabel -ExpectedLabel $Label)) {
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