param(
  [switch]$SkipCargo
)

$ErrorActionPreference = "Stop"

function Invoke-WorkflowStep {
  param(
    [string]$Name,
    [scriptblock]$Command
  )

  Write-Host ""
  Write-Host $Name
  Write-Host ("=" * $Name.Length)

  $global:LASTEXITCODE = 0

  & $Command

  if ($global:LASTEXITCODE -ne 0) {
    throw "workflow step failed: $Name"
  }
}

function Test-BenchmarkScriptPath {
  param(
    [string]$ScriptsRoot,
    [string]$RelativePath
  )

  $ScriptPath = Join-Path $ScriptsRoot $RelativePath

  if (!(Test-Path -LiteralPath $ScriptPath)) {
    throw "required benchmark script was not found: $ScriptPath"
  }
}

function Test-BenchmarkRootWrapperTarget {
  param(
    [string]$ScriptsRoot,
    [string]$WrapperName,
    [string]$ExpectedRelativeTarget
  )

  $WrapperPath = Join-Path $ScriptsRoot $WrapperName
  $ExpectedTargetPath = Join-Path $ScriptsRoot $ExpectedRelativeTarget

  if (!(Test-Path -LiteralPath $WrapperPath)) {
    throw "root benchmark wrapper was not found: $WrapperPath"
  }

  if (!(Test-Path -LiteralPath $ExpectedTargetPath)) {
    throw "root benchmark wrapper target was not found: $ExpectedTargetPath"
  }

  $WrapperText = Get-Content -Raw -LiteralPath $WrapperPath
  $ExpectedTargetPattern = [regex]::Escape($ExpectedRelativeTarget)

  if ($WrapperText -notmatch $ExpectedTargetPattern) {
    throw "root benchmark wrapper '$WrapperPath' does not point to expected target '$ExpectedRelativeTarget'"
  }
}

function Test-BenchmarkReviewRunnerContract {
  param(
    [string]$ScriptsRoot
  )

  $ReviewRunnerPath = Join-Path $ScriptsRoot "benchmark\review\run-low-gap-review.ps1"
  $UnexpectedRunPath = Join-Path $ScriptsRoot "benchmark\run\run-low-gap-review.ps1"

  if (!(Test-Path -LiteralPath $ReviewRunnerPath)) {
    throw "canonical benchmark review runner was not found: $ReviewRunnerPath"
  }

  if (Test-Path -LiteralPath $UnexpectedRunPath) {
    throw "unexpected benchmark review runner copy found under benchmark\\run: $UnexpectedRunPath"
  }

  $ReviewRunnerText = Get-Content -Raw -LiteralPath $ReviewRunnerPath

  foreach ($ExpectedParameter in @("RunNumber", "RunIndex", "RunTopic", "UpdateHistory", "HistoryDir")) {
    if ($ReviewRunnerText -notmatch ([regex]::Escape($ExpectedParameter))) {
      throw "canonical benchmark review runner is missing expected parameter or option: $ExpectedParameter"
    }
  }
}

function Test-BenchmarkScriptLayout {
  $BenchmarkRoot = Split-Path -Parent $PSScriptRoot
  $ScriptsRoot = Split-Path -Parent $BenchmarkRoot

  $RequiredCanonicalScripts = @(
    "benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1",
    "benchmark\artifacts\organize-benchmark-artifacts.ps1",
    "benchmark\check\check-benchmark-docs.ps1",
    "benchmark\check\check-benchmark-workflow.ps1",
    "benchmark\compare\compare-count-only-workload-summary.ps1",
    "benchmark\compare\compare-low-gap-trial-summary.ps1",
    "benchmark\compare\compare-low-gap-workload-summary.ps1",
    "benchmark\compare\compare-materialization-mode-summary.ps1",
    "benchmark\compare\compare-target-workload-summary.ps1",
    "benchmark\review\run-low-gap-review.ps1",
    "benchmark\review\validate-benchmark-review-artifacts.ps1",
    "benchmark\run\run-leaf-policy-trials.ps1",
    "benchmark\run\run-low-gap-trials.ps1",
    "benchmark\run\run-materialization-mode-trials.ps1",
    "benchmark\run\run-small-benchmark.ps1",
    "lib\benchmark-review\history.ps1",
    "benchmark\summarize\summarize-count-only-workload-summary.ps1",
    "benchmark\summarize\summarize-existence-timing-summary.ps1",
    "benchmark\summarize\summarize-materialization-mode-summary.ps1",
    "benchmark\summarize\summarize-typed-archive-append-rebuild-summary.ps1",
    "benchmark\summarize\summarize-typed-archive-compaction-summary.ps1",
    "benchmark\summarize\summarize-typed-archive-load-summary.ps1",
    "benchmark\summarize\summarize-typed-indexed-comparison-summary.ps1",
    "benchmark\summarize\summarize-target-workload-trials.ps1"
  )

  foreach ($RelativePath in $RequiredCanonicalScripts) {
    Test-BenchmarkScriptPath `
      -ScriptsRoot $ScriptsRoot `
      -RelativePath $RelativePath
  }

  $ExpectedRootWrappers = @{
    "cleanup-flat-benchmark-artifacts.ps1"               = "benchmark\artifacts\cleanup-flat-benchmark-artifacts.ps1"
    "organize-benchmark-artifacts.ps1"                   = "benchmark\artifacts\organize-benchmark-artifacts.ps1"
    "check-benchmark-docs.ps1"                           = "benchmark\check\check-benchmark-docs.ps1"
    "check-benchmark-workflow.ps1"                       = "benchmark\check\check-benchmark-workflow.ps1"
    "compare-count-only-workload-summary.ps1"            = "benchmark\compare\compare-count-only-workload-summary.ps1"
    "compare-low-gap-trial-summary.ps1"                  = "benchmark\compare\compare-low-gap-trial-summary.ps1"
    "compare-low-gap-workload-summary.ps1"               = "benchmark\compare\compare-low-gap-workload-summary.ps1"
    "compare-materialization-mode-summary.ps1"           = "benchmark\compare\compare-materialization-mode-summary.ps1"
    "compare-target-workload-summary.ps1"                = "benchmark\compare\compare-target-workload-summary.ps1"
    "run-low-gap-review.ps1"                             = "benchmark\review\run-low-gap-review.ps1"
    "validate-benchmark-review-artifacts.ps1"            = "benchmark\review\validate-benchmark-review-artifacts.ps1"
    "run-leaf-policy-trials.ps1"                         = "benchmark\run\run-leaf-policy-trials.ps1"
    "run-low-gap-trials.ps1"                             = "benchmark\run\run-low-gap-trials.ps1"
    "run-materialization-mode-trials.ps1"                = "benchmark\run\run-materialization-mode-trials.ps1"
    "run-small-benchmark.ps1"                            = "benchmark\run\run-small-benchmark.ps1"
    "summarize-count-only-workload-summary.ps1"          = "benchmark\summarize\summarize-count-only-workload-summary.ps1"
    "summarize-existence-timing-summary.ps1"             = "benchmark\summarize\summarize-existence-timing-summary.ps1"
    "summarize-materialization-mode-summary.ps1"         = "benchmark\summarize\summarize-materialization-mode-summary.ps1"
    "summarize-typed-archive-append-rebuild-summary.ps1" = "benchmark\summarize\summarize-typed-archive-append-rebuild-summary.ps1"
    "summarize-typed-archive-compaction-summary.ps1"     = "benchmark\summarize\summarize-typed-archive-compaction-summary.ps1"
    "summarize-typed-archive-load-summary.ps1"           = "benchmark\summarize\summarize-typed-archive-load-summary.ps1"
    "summarize-typed-indexed-comparison-summary.ps1"     = "benchmark\summarize\summarize-typed-indexed-comparison-summary.ps1"
    "summarize-target-workload-trials.ps1"               = "benchmark\summarize\summarize-target-workload-trials.ps1"
  }

  foreach ($WrapperName in $ExpectedRootWrappers.Keys) {
    Test-BenchmarkRootWrapperTarget `
      -ScriptsRoot $ScriptsRoot `
      -WrapperName $WrapperName `
      -ExpectedRelativeTarget $ExpectedRootWrappers[$WrapperName]
  }

  Test-BenchmarkReviewRunnerContract -ScriptsRoot $ScriptsRoot

  Write-Host "Benchmark script layout validation passed."
}

Write-Host "Benchmark workflow preflight"
Write-Host "============================"
Write-Host ""

Invoke-WorkflowStep `
  -Name "Benchmark documentation validation" `
  -Command {
  $DocsCheckScript = Join-Path $PSScriptRoot "check-benchmark-docs.ps1"
  & $DocsCheckScript
}

Invoke-WorkflowStep `
  -Name "Benchmark script layout validation" `
  -Command {
  Test-BenchmarkScriptLayout
}

if ($SkipCargo) {
  Write-Host ""
  Write-Host "Skipping cargo checks because -SkipCargo was provided."
  Write-Host ""
  Write-Host "Benchmark workflow preflight completed."
  exit 0
}

Invoke-WorkflowStep `
  -Name "cargo fmt check" `
  -Command {
  & cargo fmt -- --check
}

Invoke-WorkflowStep `
  -Name "cargo test" `
  -Command {
  & cargo test
}

Invoke-WorkflowStep `
  -Name "cargo check release" `
  -Command {
  & cargo check --release
}

Write-Host ""
Write-Host "Benchmark workflow preflight completed."
