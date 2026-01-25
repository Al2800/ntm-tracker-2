# MSI Validation Checklist (Windows)

This checklist captures the exact steps and results for validating the MSI build
and install/uninstall flow. Fill in the **Results** sections during the actual
Windows run.

## Environment

- Windows version: ____________________________
- Machine / VM: _______________________________
- WSL distro(s): ______________________________
- Node.js version: ____________________________
- Rust version: _______________________________
- Tauri CLI version: __________________________

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

### Results

- Build command completed: ☐ Yes / ☐ No
- MSI path(s) produced:
  - ___________________________________________
  - ___________________________________________
- Build errors (if any):
  - ___________________________________________

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

