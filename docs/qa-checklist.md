# Clean Install QA Checklist

Use this checklist for a fresh Windows 10/11 VM (no dev tools installed).

## Environment

- Windows version:
- VM image:
- WSL version:
- Distro:
- App version:
- Daemon version:

## Steps

1. Download installer from release.
2. Run installer and complete first‑run wizard.
3. Launch the app and confirm tray icon appears.
4. Verify daemon installs into WSL (`wsl.exe -- ntm-tracker-daemon --version`).
5. Create a tmux session and confirm session appears in dashboard.
6. Trigger a compact event and confirm detection.
7. Trigger an escalation and confirm notification.
8. Confirm settings persist after restart.
9. Uninstall the app.
10. Verify uninstall removes app and tray icon.

## Results

| Step | Result | Notes |
| --- | --- | --- |
| Install succeeds |  |  |
| Wizard completes |  |  |
| Daemon installs |  |  |
| Session tracking |  |  |
| Compact detection |  |  |
| Escalation detection |  |  |
| Notifications |  |  |
| Settings persistence |  |  |
| Uninstall clean |  |  |

## Issues Found

- 
