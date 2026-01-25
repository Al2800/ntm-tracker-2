# Release Signing Guide

This guide documents how to sign the MSI installer (and any update bundles) for NTM Tracker.

## Prerequisites

- Windows 10/11 host
- Windows SDK (for `signtool.exe`)
- Code-signing certificate (`.pfx`) and password
- Built MSI output (from `scripts/build.ps1`)

## Option A: Sign during build

`scripts/build.ps1` already supports signing the MSI as part of the build.

```powershell
.\scripts\build.ps1 -Profile Release -Sign `
  -PfxPath "C:\path\to\cert.pfx" `
  -PfxPassword "your-password" `
  -TimestampUrl "http://timestamp.digicert.com"
```

## Option B: Sign after build

Use the dedicated signing script. By default it signs the release MSI output directory.

```powershell
.\scripts\sign.ps1 `
  -PfxPath "C:\path\to\cert.pfx" `
  -PfxPassword "your-password"
```

To sign a custom file or glob:

```powershell
.\scripts\sign.ps1 `
  -InputPath "C:\path\to\bundle\*.msi" `
  -PfxPath "C:\path\to\cert.pfx" `
  -PfxPassword "your-password"
```

## Verify a signature

```powershell
.\scripts\sign.ps1 -InputPath "C:\path\to\bundle\ntm-tracker.msi" -VerifyOnly
```

## Notes

- Use a timestamp server so signatures remain valid after certificate expiry.
- Store the `.pfx` securely; avoid committing secrets to the repo.
- If SmartScreen warnings persist, verify the cert is EV/OV and the signature chain is intact.
