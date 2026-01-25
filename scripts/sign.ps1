$ErrorActionPreference = "Stop"

param(
  [string]$InputPath,
  [string]$PfxPath,
  [string]$PfxPassword,
  [string]$TimestampUrl = "http://timestamp.digicert.com",
  [switch]$VerifyOnly
)

function Require-Command([string]$Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing required command: $Name"
  }
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $InputPath) {
  $InputPath = Join-Path $RepoRoot "app/src-tauri/target/release/bundle/msi/*.msi"
}

Require-Command "signtool.exe"

$Targets = Get-ChildItem -Path $InputPath -ErrorAction SilentlyContinue
if (-not $Targets) {
  throw "No files matched: $InputPath"
}

Write-Host "Repo: $RepoRoot"
Write-Host "Signing targets:"
$Targets | ForEach-Object { Write-Host " - $($_.FullName)" }

if (-not $VerifyOnly) {
  if (-not $PfxPath) { throw "Missing -PfxPath for signing" }
  if (-not $PfxPassword) { throw "Missing -PfxPassword for signing" }
}

foreach ($file in $Targets) {
  if (-not $VerifyOnly) {
    Write-Host "Signing: $($file.FullName)"
    & signtool.exe sign `
      /fd SHA256 `
      /f $PfxPath `
      /p $PfxPassword `
      /tr $TimestampUrl `
      /td SHA256 `
      $file.FullName
  }

  Write-Host "Verifying: $($file.FullName)"
  & signtool.exe verify /pa $file.FullName
}

Write-Host "Signing/verification complete."
