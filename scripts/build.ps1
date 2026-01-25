$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  npm -C app run build
  cargo tauri build --manifest-path app/src-tauri/Cargo.toml --bundles msi

  if ($env:NTM_SIGNTOOL -and $env:NTM_SIGN_CERT -and $env:NTM_SIGN_PASSWORD) {
    $msiPath = Get-ChildItem -Path app/src-tauri/target/release/bundle/msi -Filter "*.msi" | Select-Object -First 1
    if ($msiPath) {
      & $env:NTM_SIGNTOOL sign /f $env:NTM_SIGN_CERT /p $env:NTM_SIGN_PASSWORD /tr http://timestamp.digicert.com /td sha256 /fd sha256 $msiPath.FullName
    }
  }
} finally {
  Pop-Location
}
