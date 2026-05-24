param(
  [string]$DocsRoot = "dev_notes/benchmark",
  [string]$IndexPath = "dev_notes/benchmark_acceptance.md"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($DocsRoot)) {
  throw "-DocsRoot cannot be empty"
}

if ([string]::IsNullOrWhiteSpace($IndexPath)) {
  throw "-IndexPath cannot be empty"
}

$Failures = New-Object System.Collections.Generic.List[string]

function Add-Failure {
  param(
    [string]$Message
  )

  $Failures.Add($Message) | Out-Null
}

if (!(Test-Path -LiteralPath $DocsRoot)) {
  throw "benchmark docs root was not found: $DocsRoot"
}

$DocsRootPath = (Resolve-Path -LiteralPath $DocsRoot).Path

if (Test-Path -LiteralPath $IndexPath) {
  $ResolvedIndexPath = (Resolve-Path -LiteralPath $IndexPath).Path
}
else {
  $ResolvedIndexPath = ""
  Add-Failure "benchmark notes index was not found: $IndexPath"
}

$RequiredTopicFiles = @(
  "README.md",
  "acceptance-rules.md",
  "datasets-and-targets.md",
  "diagnostic-conclusions.md",
  "count-only-workflow.md",
  "review-workflow.md",
  "artifact-workflow.md",
  "leaf-policy.md"
)

$IndexedTopicFiles = @(
  "acceptance-rules.md",
  "datasets-and-targets.md",
  "diagnostic-conclusions.md",
  "count-only-workflow.md",
  "review-workflow.md",
  "artifact-workflow.md",
  "leaf-policy.md"
)

foreach ($TopicFile in $RequiredTopicFiles) {
  $TopicPath = Join-Path $DocsRootPath $TopicFile

  if (!(Test-Path -LiteralPath $TopicPath)) {
    Add-Failure "required benchmark topic file was not found: $TopicPath"
  }
}

$MarkdownFiles = @(
  Get-ChildItem -LiteralPath $DocsRootPath -Filter "*.md" -File |
  Sort-Object -Property FullName
)

if ($MarkdownFiles.Count -eq 0) {
  Add-Failure "no markdown files were found in benchmark docs root: $DocsRootPath"
}

$FilesToCheck = New-Object System.Collections.Generic.List[System.IO.FileInfo]

if (![string]::IsNullOrWhiteSpace($ResolvedIndexPath)) {
  $FilesToCheck.Add((Get-Item -LiteralPath $ResolvedIndexPath)) | Out-Null
}

foreach ($MarkdownFile in $MarkdownFiles) {
  $FilesToCheck.Add($MarkdownFile) | Out-Null
}

$Backtick = [char]96
$Fence3 = "$Backtick$Backtick$Backtick"
$Fence4 = "$Backtick$Backtick$Backtick$Backtick"

foreach ($File in $FilesToCheck) {
  $Lines = [System.IO.File]::ReadAllLines($File.FullName)

  $OpenFenceLine = 0
  $FenceCount = 0

  for ($Index = 0; $Index -lt $Lines.Count; $Index++) {
    $Line = $Lines[$Index]
    $TrimmedLine = $Line.TrimStart()
    $LineNumber = $Index + 1

    if ($TrimmedLine.StartsWith($Fence4)) {
      Add-Failure ("{0}:{1} malformed markdown fence uses more than three backticks" -f $File.FullName, $LineNumber)
      continue
    }

    if ($TrimmedLine.StartsWith($Fence3)) {
      $FenceCount += 1

      if ($OpenFenceLine -eq 0) {
        $OpenFenceLine = $LineNumber
      }
      else {
        $OpenFenceLine = 0
      }
    }
  }

  if ($OpenFenceLine -ne 0) {
    Add-Failure ("{0}:{1} unclosed markdown fence" -f $File.FullName, $OpenFenceLine)
  }

  if (($FenceCount % 2) -ne 0) {
    Add-Failure ("{0} has an odd number of markdown fence lines: {1}" -f $File.FullName, $FenceCount)
  }
}

if (![string]::IsNullOrWhiteSpace($ResolvedIndexPath)) {
  $IndexText = [System.IO.File]::ReadAllText($ResolvedIndexPath)

  foreach ($TopicFile in $IndexedTopicFiles) {
    $ExpectedReference = "dev_notes/benchmark/$TopicFile"

    if (!$IndexText.Contains($ExpectedReference)) {
      Add-Failure ("benchmark notes index does not reference required topic file: {0}" -f $ExpectedReference)
    }
  }
}

$ReadmePath = Join-Path $DocsRootPath "README.md"

if (Test-Path -LiteralPath $ReadmePath) {
  $ReadmeText = [System.IO.File]::ReadAllText($ReadmePath)

  foreach ($TopicFile in $IndexedTopicFiles) {
    if (!$ReadmeText.Contains($TopicFile)) {
      Add-Failure ("benchmark README does not mention required topic file: {0}" -f $TopicFile)
    }
  }
}

Write-Host "Benchmark markdown and index check"
Write-Host "=================================="
Write-Host ""
Write-Host "Docs root:           $DocsRootPath"
Write-Host "Index path:          $IndexPath"
Write-Host "Markdown files:      $($MarkdownFiles.Count)"
Write-Host "Files checked:       $($FilesToCheck.Count)"
Write-Host "Required topics:     $($RequiredTopicFiles.Count)"
Write-Host "Failures:            $($Failures.Count)"
Write-Host ""

if ($Failures.Count -gt 0) {
  Write-Host "Benchmark docs validation failures:"
  Write-Host ""

  foreach ($Failure in $Failures) {
    Write-Host "  $Failure"
  }

  throw "benchmark docs validation failed"
}

Write-Host "Benchmark docs validation passed."