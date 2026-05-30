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

function Invoke-BenchmarkReviewPolicyCsvValidation {
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

  Read-BenchmarkReviewPolicyValidatedCsv `
    -Entry $Entry `
    -Description $Description `
    -RequiredColumns $RequiredColumns `
    -MinimumRows $MinimumRows `
    -BooleanTrueColumns $BooleanTrueColumns `
    -RequireBaselines $RequireBaselines `
    -TrialCountColumn $TrialCountColumn `
    -ExpectedTrials $ExpectedTrials `
    -OnFailure $OnFailure | Out-Null
}

function Write-BenchmarkReviewValidationReport {
  param(
    [string]$Label,
    [string]$OutputDir,
    [string]$ManifestPath,
    [int]$ExpectedTrials,
    [bool]$RequireComparisons,
    [bool]$AllowSkippedTargetWorkloadReview,
    [int]$ManifestEntryCount,
    [string[]]$Failures = @()
  )

  Write-Host "Benchmark review artifact validation"
  Write-Host "===================================="
  Write-Host ""
  Write-Host "Label:                     $Label"
  Write-Host "Output dir:                $OutputDir"
  Write-Host "Manifest:                  $ManifestPath"
  Write-Host "Expected trials:           $ExpectedTrials"
  Write-Host "Require comparisons:       $RequireComparisons"
  Write-Host "Allow skipped target:      $AllowSkippedTargetWorkloadReview"
  Write-Host "Manifest artifacts loaded: $ManifestEntryCount"
  Write-Host "Failures:                  $($Failures.Count)"
  Write-Host ""

  if ($Failures.Count -gt 0) {
    Write-Host "Validation failures:"
    Write-Host ""

    foreach ($Failure in $Failures) {
      Write-Host "  $Failure"
    }
  }
}
