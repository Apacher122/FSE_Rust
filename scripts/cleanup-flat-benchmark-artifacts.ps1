param(
  [string]$ArtifactRoot = "benchmark_artifacts",
  [string]$RunRoot = "",
  [string]$Label = "",
  [switch]$Delete,
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

if (!(Test-Path -LiteralPath $RunRoot)) {
  throw "organized run root was not found: $RunRoot"
}

$ArtifactRootPath = (Resolve-Path -LiteralPath $ArtifactRoot).Path
$RunRootPath = (Resolve-Path -LiteralPath $RunRoot).Path

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

function Get-OrganizedArtifactPath {
  param(
    [string]$DiscoveredLabel,
    [System.IO.FileSystemInfo]$Item
  )

  $RunDirectory = Join-Path $RunRootPath $DiscoveredLabel
  return Join-Path $RunDirectory $Item.Name
}

function Remove-FlatArtifact {
  param(
    [System.IO.FileSystemInfo]$Item
  )

  if (!$Delete) {
    Write-Host "DRY RUN: remove `"$($Item.FullName)`""
    return
  }

  Remove-Item `
    -LiteralPath $Item.FullName `
    -Recurse `
    -Force:$Force

  Write-Host "removed: $($Item.FullName)"
}

$Items = Get-ChildItem -LiteralPath $ArtifactRootPath -Force

$RecognizedItems = New-Object System.Collections.Generic.List[object]
$EligibleItems = New-Object System.Collections.Generic.List[object]
$SkippedItems = New-Object System.Collections.Generic.List[object]
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

  $OrganizedPath = Get-OrganizedArtifactPath `
    -DiscoveredLabel $DiscoveredLabel `
    -Item $Item

  $Entry = [PSCustomObject]@{
    Item                = $Item
    Label               = $DiscoveredLabel
    OrganizedPath       = $OrganizedPath
    OrganizedCopyExists = Test-Path -LiteralPath $OrganizedPath
  }

  $RecognizedItems.Add($Entry)

  if ($Entry.OrganizedCopyExists) {
    $EligibleItems.Add($Entry)
  }
  else {
    $SkippedItems.Add($Entry)
  }
}

Write-Host "Flat benchmark artifact cleanup"
Write-Host "==============================="
Write-Host ""
Write-Host "Artifact root: $ArtifactRootPath"
Write-Host "Run root:      $RunRootPath"
Write-Host "Mode:          $(if ($Delete) { "delete" } else { "dry run" })"
Write-Host "Force:         $Force"

if (![string]::IsNullOrWhiteSpace($Label)) {
  Write-Host "Label filter:  $Label"
}

Write-Host ""
Write-Host "Recognized flat artifacts:       $($RecognizedItems.Count)"
Write-Host "Eligible for cleanup:            $($EligibleItems.Count)"
Write-Host "Skipped without organized copy:  $($SkippedItems.Count)"
Write-Host "Unrecognized top-level items:    $($UnrecognizedItems.Count)"
Write-Host ""

if ($EligibleItems.Count -eq 0) {
  Write-Host "No flat artifacts are eligible for cleanup."

  if ($SkippedItems.Count -gt 0) {
    Write-Host ""
    Write-Host "Some artifacts were skipped because no organized copy exists."
    Write-Host "Run organize-benchmark-artifacts.ps1 -Copy first if you want to clean them safely."
  }

  return
}

$GroupedEligibleItems = $EligibleItems |
Group-Object -Property Label |
Sort-Object -Property Name

foreach ($Group in $GroupedEligibleItems) {
  Write-Host "Label: $($Group.Name)"

  foreach ($Entry in ($Group.Group | Sort-Object { $_.Item.Name })) {
    Write-Host "  flat:      $($Entry.Item.FullName)"
    Write-Host "  organized: $($Entry.OrganizedPath)"

    Remove-FlatArtifact -Item $Entry.Item
  }

  Write-Host ""
}

Write-Host "Cleanup summary"
Write-Host "---------------"
Write-Host "Labels touched:                 $($GroupedEligibleItems.Count)"
Write-Host "Flat artifacts eligible:         $($EligibleItems.Count)"
Write-Host "Flat artifacts skipped:          $($SkippedItems.Count)"
Write-Host "Unrecognized top-level items:    $($UnrecognizedItems.Count)"

if ($SkippedItems.Count -gt 0) {
  Write-Host ""
  Write-Host "Skipped artifacts without organized copies:"
    
  foreach ($Entry in ($SkippedItems | Sort-Object -Property Label)) {
    Write-Host "  $($Entry.Item.Name)"
  }
}

if ($UnrecognizedItems.Count -gt 0) {
  Write-Host ""
  Write-Host "Unrecognized top-level items were left in place:"

  foreach ($Item in ($UnrecognizedItems | Sort-Object -Property Name)) {
    Write-Host "  $($Item.Name)"
  }
}

if (!$Delete) {
  Write-Host ""
  Write-Host "Dry run only. Pass -Delete to remove eligible flat artifacts."
  Write-Host "The script only deletes artifacts that already exist in the organized run folder."
}