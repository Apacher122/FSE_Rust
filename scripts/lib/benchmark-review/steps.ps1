function Get-BenchmarkReviewComparisonSkipReason {
  param(
    [string]$PreviousPath,
    [string]$CurrentPath,
    [string]$PreviousDescription,
    [string]$CurrentDescription,
    [bool]$Disabled = $false,
    [string]$DisabledReason = ""
  )

  if ($Disabled) {
    if ([string]::IsNullOrWhiteSpace($DisabledReason)) {
      return "$CurrentDescription was disabled"
    }

    return $DisabledReason
  }

  if ([string]::IsNullOrWhiteSpace($PreviousPath)) {
    return "$PreviousDescription was not provided"
  }

  if (!(Test-BenchmarkReviewPathExists -Path $PreviousPath)) {
    return "$PreviousDescription was not found"
  }

  if (!(Test-BenchmarkReviewPathExists -Path $CurrentPath)) {
    return "$CurrentDescription was not found"
  }

  return ""
}

function Invoke-BenchmarkReviewComparisonStep {
  param(
    [string]$ReviewNotesPath,
    [string]$HostTitle,
    [string]$ReviewTitle,
    [string]$ReviewUnderline,
    [string]$PreviousPath,
    [string]$CurrentPath,
    [string]$PreviousDescription,
    [string]$CurrentDescription,
    [string]$ScriptPath,
    [hashtable]$ScriptArguments,
    [string]$FailureMessage,
    [string[]]$CompletedLines,
    [bool]$Disabled = $false,
    [string]$DisabledConsoleReason = "",
    [string]$DisabledReviewReason = ""
  )

  Write-Host ""
  Write-Host "Running $HostTitle"

  Add-Utf8Text -Path $ReviewNotesPath -Text "$ReviewTitle`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "$ReviewUnderline`r`n"

  $SkipReason = Get-BenchmarkReviewComparisonSkipReason `
    -PreviousPath $PreviousPath `
    -CurrentPath $CurrentPath `
    -PreviousDescription $PreviousDescription `
    -CurrentDescription $CurrentDescription `
    -Disabled $Disabled `
    -DisabledReason $DisabledConsoleReason

  if (![string]::IsNullOrWhiteSpace($SkipReason)) {
    Write-Host "  skipped: $SkipReason"

    if (!$Disabled -and ![string]::IsNullOrWhiteSpace($PreviousPath) -and !(Test-BenchmarkReviewPathExists -Path $PreviousPath)) {
      Write-Host "  $PreviousPath"
    }
    elseif (!$Disabled -and ![string]::IsNullOrWhiteSpace($PreviousPath) -and (Test-BenchmarkReviewPathExists -Path $PreviousPath) -and !(Test-BenchmarkReviewPathExists -Path $CurrentPath)) {
      Write-Host "  $CurrentPath"
    }

    $ReviewReason = $SkipReason

    if ($Disabled -and ![string]::IsNullOrWhiteSpace($DisabledReviewReason)) {
      $ReviewReason = $DisabledReviewReason
    }

    Add-Utf8Text -Path $ReviewNotesPath -Text "status: skipped`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "reason: $ReviewReason`r`n"
    Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"

    return
  }

  & $ScriptPath @ScriptArguments

  if ($LASTEXITCODE -ne 0) {
    throw "$FailureMessage $LASTEXITCODE"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "status: completed`r`n"

  foreach ($CompletedLine in $CompletedLines) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "$CompletedLine`r`n"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
}

function Invoke-BenchmarkReviewValidationStep {
  param(
    [string]$ReviewNotesPath,
    [string]$Label,
    [string]$OutputDir,
    [int]$ExpectedTrials,
    [bool]$RequireValidatedComparisons,
    [bool]$AllowSkippedTargetWorkloadReview,
    [string]$ArtifactValidatorScript
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Review artifact validation`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "--------------------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "status: running`r`n"

  Write-Host ""
  Write-Host "Validating review artifacts"
  Write-Host "  label: $Label"

  $ValidatorArguments = @{
    Label          = $Label
    OutputDir      = $OutputDir
    ExpectedTrials = $ExpectedTrials
  }

  if ($RequireValidatedComparisons) {
    $ValidatorArguments["RequireComparisons"] = $true
  }

  if ($AllowSkippedTargetWorkloadReview) {
    $ValidatorArguments["AllowSkippedTargetWorkloadReview"] = $true
  }

  & $ArtifactValidatorScript @ValidatorArguments

  if ($LASTEXITCODE -ne 0) {
    Add-Utf8Text -Path $ReviewNotesPath -Text "status: failed`r`n"
    throw "review artifact validation failed with exit code $LASTEXITCODE"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "status: completed`r`n"
}

function Copy-BenchmarkReviewNotesToOrganizedRun {
  param(
    [string]$ReviewNotesPath,
    [string]$OrganizedRunDirectory,
    [bool]$CopyArtifactsToRunFolder
  )

  if (!$CopyArtifactsToRunFolder) {
    return
  }

  New-Item -ItemType Directory -Force -Path $OrganizedRunDirectory | Out-Null

  Copy-Item `
    -LiteralPath $ReviewNotesPath `
    -Destination (Join-Path $OrganizedRunDirectory (Split-Path -Leaf $ReviewNotesPath)) `
    -Force
}

function Invoke-BenchmarkReviewOrganizedArtifactCopy {
  param(
    [string]$ReviewNotesPath,
    [string]$OutputDir,
    [string]$Label,
    [string]$OrganizedRunDirectory,
    [bool]$ForceOrganizedArtifacts,
    [string]$ArtifactOrganizerScript
  )

  Add-Utf8Text -Path $ReviewNotesPath -Text "`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "Organized artifact copy`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "-----------------------`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "status: running`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "destination: $OrganizedRunDirectory`r`n"
  Add-Utf8Text -Path $ReviewNotesPath -Text "force: $ForceOrganizedArtifacts`r`n"

  Write-Host ""
  Write-Host "Copying current review artifacts to organized run folder"
  Write-Host "  label: $Label"
  Write-Host "  destination: $OrganizedRunDirectory"

  $OrganizerArguments = @{
    ArtifactRoot = $OutputDir
    Label        = $Label
    Copy         = $true
  }

  if ($ForceOrganizedArtifacts) {
    $OrganizerArguments["Force"] = $true
  }

  & $ArtifactOrganizerScript @OrganizerArguments

  if ($LASTEXITCODE -ne 0) {
    throw "organized artifact copy failed with exit code $LASTEXITCODE"
  }

  Add-Utf8Text -Path $ReviewNotesPath -Text "status: completed`r`n"

  Copy-BenchmarkReviewNotesToOrganizedRun `
    -ReviewNotesPath $ReviewNotesPath `
    -OrganizedRunDirectory $OrganizedRunDirectory `
    -CopyArtifactsToRunFolder $true
}
