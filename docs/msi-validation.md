# MSI Validation Checklist (Windows)

This checklist captures the exact steps and results for validating the MSI build
and install/uninstall flow. Fill in the **Results** sections during the actual
Windows run.

## Environment

- Windows version: Windows 11 Build 26200
- Machine / VM: Native
- WSL distro(s): Ubuntu (WSL 2.6.1.0)
- Node.js version: v22.20.0
- Rust version: 1.91.1
- npm version: 11.6.2

## Build

### Prereqs

- `npm` and `cargo` available on PATH
- Tauri build deps installed
- Optional signing: `signtool.exe` + code-signing certificate

### Command

```powershell
./scripts/build.ps1 -Profile Release
```

Optional signing:

```powershell
./scripts/build.ps1 -Profile Release -Sign -PfxPath "C:\path\cert.pfx" -PfxPassword "***"
```

### Results (2026-01-25)

- Build command completed: ☒ No (Rust compilation failed)
- MSI path(s) produced: None
- Frontend build: ✅ Succeeded (Svelte + Vite)
- Backend build: ❌ Failed (Tauri Rust)

**Rust Compilation Errors:**
1. `resolver::Resolver` - wrong import path
2. `BundleDirNotFound` - missing error type
3. `path_resolver` method not found (Tauri 2 API change)
4. `IntoClientRequest` trait not implemented for `tauri::Url`
5. WebSocket `Message::Text` type mismatches (Utf8Bytes vs String)

**Root Cause:** Tauri app code has API incompatibilities with Tauri 2.x.

**Fixes Required (see bd-2ml.4.15.1):**
- Update commands.rs for Tauri 2 path resolver API
- Fix resolver imports
- Update WebSocket code for tungstenite 0.27+ API changes

## Install

### Steps

1. Double‑click the MSI from `app/src-tauri/target/release/bundle/msi/`.
2. Complete the install wizard with defaults.
3. Launch the app from Start Menu.
4. Confirm tray icon appears and first‑run daemon bootstrap triggers.

### Results

- Install succeeded: ☐ Yes / ☐ No
- Tray icon visible: ☐ Yes / ☐ No
- Daemon bootstrap completed: ☐ Yes / ☐ No
- Errors / dialogs:
  - ___________________________________________

## Uninstall

### Steps

1. Settings → Apps → NTM Tracker → Uninstall.
2. Confirm files removed from Program Files.
3. Launch after uninstall to verify clean removal (should fail gracefully).

### Results

- Uninstall succeeded: ☐ Yes / ☐ No
- No leftover files in Program Files: ☐ Yes / ☐ No
- WSL daemon cleaned up (if expected): ☐ Yes / ☐ No / ☐ N/A
- Errors / dialogs:
  - ___________________________________________

## Notes

- Observed MSI metadata (publisher, version, upgrade code):
  - ___________________________________________
- SmartScreen behavior:
  - ___________________________________________
- Additional issues / follow‑ups:
  - ___________________________________________

