$ErrorActionPreference = "Stop"

param(
  [ValidateSet("Debug", "Release")]
  [string]$Profile = "Release",

  [switch]$Sign,

  [string]$PfxPath,
  [string]$PfxPassword,
  [string]$TimestampUrl = "http://timestamp.digicert.com"
)

function Require-Command([string]$Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing required command: $Name"
  }
}

Require-Command "npm"
Require-Command "cargo"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$AppDir = Join-Path $RepoRoot "app"

Write-Host "Repo: $RepoRoot"
Write-Host "App:  $AppDir"

Push-Location $AppDir
try {
  Write-Host "Installing Node deps..."
  npm ci

  Write-Host "Building MSI via Tauri CLI..."
  if ($Profile -eq "Release") {
    npm run tauri build -- --bundles msi
  } else {
    npm run tauri build -- --bundles msi --debug
  }
} finally {
  Pop-Location
}

$BundleDir = Join-Path $RepoRoot "app/src-tauri/target"
$MsiGlob = Join-Path $BundleDir "release/bundle/msi/*.msi"
if ($Profile -ne "Release") {
  $MsiGlob = Join-Path $BundleDir "debug/bundle/msi/*.msi"
}

$Msis = Get-ChildItem -Path $MsiGlob -ErrorAction SilentlyContinue
if (-not $Msis) {
  throw "MSI not found at: $MsiGlob"
}

Write-Host "Built MSI:"
$Msis | ForEach-Object { Write-Host " - $($_.FullName)" }

if ($Sign) {
  Require-Command "signtool.exe"
  if (-not $PfxPath) { throw "Missing -PfxPath for signing" }
  if (-not $PfxPassword) { throw "Missing -PfxPassword for signing" }

  foreach ($msi in $Msis) {
    Write-Host "Signing: $($msi.FullName)"
    & signtool.exe sign `
      /fd SHA256 `
      /f $PfxPath `
      /p $PfxPassword `
      /tr $TimestampUrl `
      /td SHA256 `
      $msi.FullName
  }
}

