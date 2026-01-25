# Windows App Crash-Recovery Checklist

Manual test plan for validating crash recovery scenarios.

## Environment

- Windows version:
- WSL distro:
- App version:
- Daemon version:

## Scenario 1 — Crash while connected

1. Start the app and confirm **Connected** status.
2. Force-terminate the app (Task Manager → End task).
3. Relaunch the app.

**Expected**
- App reconnects automatically.
- Sessions/panes populate within 5–10 seconds.
- No stale “Disconnected” banner.

## Scenario 2 — Crash during notification

1. Trigger a compact/escalation notification (use fixture or real session).
2. While toast is visible, force-terminate the app.
3. Relaunch the app.

**Expected**
- Pending escalations still appear in dashboard list.
- No duplicate notifications on restart (unless configured).

## Scenario 3 — Settings persistence

1. Change a setting (theme, notifications, transport).
2. Force-terminate the app.
3. Relaunch the app.

**Expected**
- Settings persist and apply immediately.

## Scenario 4 — Single-instance enforcement

1. Launch the app.
2. Attempt to launch a second instance (double‑click).

**Expected**
- Existing window is focused.
- No duplicate tray icons.

## Results

| Scenario | Result | Notes |
| --- | --- | --- |
| Crash while connected | Pass / Fail | |
| Crash during notification | Pass / Fail | |
| Settings persistence | Pass / Fail | |
| Single‑instance | Pass / Fail | |
