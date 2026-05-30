$BenchmarkReviewLibraryRoot = Join-Path $PSScriptRoot "benchmark-review"

$BenchmarkReviewLibraryFiles = @(
  "model.ps1",
  "manifest.ps1",
  "steps.ps1",
  "notes.ps1",
  "validation.ps1",
  "emission.ps1"
)

foreach ($BenchmarkReviewLibraryFile in $BenchmarkReviewLibraryFiles) {
  $BenchmarkReviewLibraryPath = Join-Path $BenchmarkReviewLibraryRoot $BenchmarkReviewLibraryFile

  if (!(Test-Path -LiteralPath $BenchmarkReviewLibraryPath)) {
    throw "benchmark review helper library was not found: $BenchmarkReviewLibraryPath"
  }

  . $BenchmarkReviewLibraryPath
}
