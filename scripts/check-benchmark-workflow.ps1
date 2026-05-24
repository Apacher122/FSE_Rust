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

  & $Command

  if ($LASTEXITCODE -ne 0) {
    throw "workflow step failed: $Name"
  }
}

Write-Host "Benchmark workflow preflight"
Write-Host "============================"
Write-Host ""

Invoke-WorkflowStep `
  -Name "Benchmark documentation validation" `
  -Command {
  & ".\scripts\check-benchmark-docs.ps1"
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