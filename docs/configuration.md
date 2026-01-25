# Configuration Reference

This document covers the daemon configuration file, defaults, and environment overrides.

## Config File Locations

The daemon loads `daemon.toml` from the first location that exists:

1. `$XDG_CONFIG_HOME/ntm-tracker/daemon.toml`
2. `$HOME/.config/ntm-tracker/daemon.toml`
3. `/etc/ntm-tracker/daemon.toml`

If no file exists, defaults are used.

## Example `daemon.toml`

```toml
[server]
bind = "127.0.0.1:3847"

[polling]
snapshot-interval-ms = 2000

[capture]
capture-output = false

[security]
# Optional: path to admin token file (Unix permissions must be 0600)
admin-token-path = "/home/user/.config/ntm-tracker/admin.token"

[privacy]
# Regex patterns used to redact sensitive output
redaction-patterns = ["(?i)sk-[a-z0-9]+", "(?i)api_key=\\w+"]

[logging]
level = "info"
# file = "/home/user/.local/share/ntm-tracker/daemon.log"
max-file-mb = 10
max-files = 5
format = "text"
```

## Settings Reference

### `server`
- `bind` (string, default `127.0.0.1:3847`)
  - Address/port to bind for optional HTTP/WS service mode.

### `polling`
- `snapshot-interval-ms` (u64, default `2000`)
  - Full snapshot poll interval in milliseconds.
  - Valid range: **250–60000**.

### `capture`
- `capture-output` (bool, default `false`)
  - When `true`, enables pane output capture (use with care; privacy risk).

### `security`
- `admin-token-path` (string, optional)
  - Path to admin token file.
  - On Unix, file permissions must be **0600**.

### `privacy`
- `redaction-patterns` (string array, default `[]`)
  - Regex patterns used to redact sensitive output.
  - Invalid regexes fail config validation.

### `logging`
- `level` (string, default `info`)
  - One of `trace`, `debug`, `info`, `warn`, `error`.
- `file` (string, optional)
  - Log file path. If omitted, logs to stdout only.
- `max-file-mb` (u64, default `10`)
  - Max log file size before rotation.
- `max-files` (usize, default `5`)
  - Number of rotated log files to keep.
- `format` (string, default `text`)
  - `text` or `json`.

## Environment Overrides

Environment variables override config file values:

| Environment variable | Setting |
| --- | --- |
| `NTM_TRACKER_SERVER_BIND` | `server.bind` |
| `NTM_TRACKER_POLLING_SNAPSHOT_INTERVAL_MS` | `polling.snapshot-interval-ms` |
| `NTM_TRACKER_CAPTURE_OUTPUT` | `capture.capture-output` (`1/true/yes/on` = true) |
| `NTM_TRACKER_PRIVACY_REDACTION_PATTERNS` | `privacy.redaction-patterns` (comma‑separated) |
| `NTM_TRACKER_SECURITY_ADMIN_TOKEN_PATH` | `security.admin-token-path` |

## Reloading Configuration

On Unix platforms, sending `SIGHUP` triggers a config reload. The daemon will keep
running with the last known-good configuration if reload validation fails.
