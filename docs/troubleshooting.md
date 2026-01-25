# Troubleshooting Guide

Use this guide to diagnose connection issues, missing data, or instability.

## Quick Checks

1. **Is WSL running?**
   - From PowerShell: `wsl.exe --status`
2. **Is tmux installed inside WSL?**
   - `wsl.exe -d <distro> -- tmux -V`
3. **Is NTM installed (optional)?**
   - `wsl.exe -d <distro> -- ntm --version`
4. **Restart the daemon**
   - If using stdio mode, just restart the Windows app.
   - If running as a service, restart the systemd unit (if configured).

## Connection Errors

### Symptoms
- App shows “Disconnected” or “No daemon detected”.
- No session updates appear.

### Fixes
- **Prefer stdio mode**: If WS/HTTP is flaky, use stdio (default) to avoid WSL localhost forwarding issues.
- **Restart WSL**:
  ```powershell
  wsl.exe --shutdown
  ```
- **VPN / firewall interference**: Temporarily disable VPN or firewall to test TCP connectivity.
- **Port conflicts**: Ensure `127.0.0.1:3847` is free if using WS/HTTP.
- **Unauthorized errors**: If `security.admin-token-path` is set, send
  `Authorization: Bearer <token>` (or `?auth=<token>` for WebSocket handshakes).

## No Sessions Showing

### Symptoms
- App is connected but shows no sessions.

### Fixes
- Confirm tmux is running inside WSL:
  ```bash
  tmux ls
  ```
- Ensure tmux is using the default socket. If you use a custom socket, configure it (future enhancement).
- If `ntm` is not installed, the daemon runs in **tmux‑only** mode.

## High Latency / Slow Updates

### Symptoms
- Updates lag behind changes in tmux sessions.

### Fixes
- Check polling interval in `daemon.toml`:
  - `polling.snapshot-interval-ms` default is 2000ms.
- Ensure the WSL environment isn’t under heavy CPU load.
- Reduce expensive NTM sections (use `--md-sections sessions` if running custom scripts).

## Log Collection

Enable file logging in `daemon.toml`:

```toml
[logging]
level = "debug"
file = "/home/user/.local/share/ntm-tracker/daemon.log"
format = "text"
```

Then reproduce the issue and attach the log file to your report.

## Configuration Validation Errors

### Symptoms
- Daemon exits immediately with a configuration error.

### Fixes
- Ensure `polling.snapshot-interval-ms` is between 250 and 60000.
- Validate regexes in `privacy.redaction-patterns`.
- If `security.admin-token-path` is set on Unix, ensure permissions are `0600`.

## Daemon Upgrade Rollback

If a daemon upgrade fails a health check, the app restores the previous binary
automatically.

### Manual rollback (WSL)

If you need to restore manually, run:

```powershell
wsl.exe -- sh -lc "mv ~/.local/bin/ntm-tracker-daemon.backup ~/.local/bin/ntm-tracker-daemon"
```

You can verify the restored version with:

```powershell
wsl.exe -- ntm-tracker-daemon --version
```

## WSL Networking Notes (WS/HTTP)

WSL localhost forwarding can be unreliable on some machines. If you must use WS/HTTP:

- Bind to `127.0.0.1:3847` only.
- Avoid VPNs while testing.
- Restart WSL after resume/sleep.
- If connections hang after sleep/VPN changes, run `wsl.exe --shutdown` and relaunch.
- Verify the port is reachable from Windows:
  ```powershell
  Test-NetConnection -ComputerName 127.0.0.1 -Port 3847
  ```
- If TCP fails repeatedly, switch transport back to **stdio** (default) in settings.

## Still Stuck?

Collect the following before filing an issue:

- Windows version and WSL distro
- App + daemon version
- `daemon.toml` (redact secrets)
- Recent log excerpt
- Repro steps
