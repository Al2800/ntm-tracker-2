# Fixture Corpus

This folder contains captured (or synthetic) pane output used for parser and detector tests.

## Layout

- `fixtures/sessions/*.json`
  - Session snapshots with pane output lines.
  - Schema (informal):
    ```json
    {
      "session": "name",
      "captured_at": "RFC3339 timestamp",
      "description": "optional",
      "panes": {
        "0": { "type": "claude|codex|unknown", "state": "active|idle|waiting", "lines": ["..."], "truncated": false }
      }
    }
    ```
- `fixtures/expected/*.expected.json`
  - Expected detector hits derived from the fixture lines.
  - Schema (informal):
    ```json
    {
      "schema_version": 1,
      "fixture": "<fixture file>",
      "session": "name",
      "panes": {
        "0": {
          "compacts": [{"line_contains": "...", "reason": "compacting"}],
          "escalations": [{"line_contains": "...", "severity": "warn"}],
          "status_samples": [{"line_contains": "...", "expected_status": "waiting", "reason": "waiting_pattern"}]
        }
      }
    }
    ```
  - `status-snapshot.expected.json` is a special case for the JSON status snapshot.
  - Legacy files ending in `-expected.json` are older schemas kept for reference.

## Current Fixtures

- `ansi-heavy-sample.json` — ANSI escape sequences, no detector hits expected.
- `compact-trigger-sample.json` — Compact warning + auto-compacting line.
- `cuas-sim-claude.json` — Real claude session capture (parser robustness).
- `escalation-sample.json` — Escalation prompts + fatal error line.
- `ntracker3-codex.json` — Large codex session capture.
- `shell-only-sample.json` — Shell-only output, no AI patterns.
- `speedread-ios-mixed.json` — Mixed session capture.
- `status-snapshot.json` — NTM status JSON snapshot (metadata parsing).
- `status-transitions-sample.json` — Status detection patterns.
- `unicode-sample.json` — Unicode + emoji output.

## Adding New Fixtures

1. Capture a pane snapshot into `fixtures/sessions/<name>.json`.
2. Add a matching `fixtures/expected/<name>.expected.json` with detector expectations.
3. Update this README with a short description.

## Notes

- These fixtures are **redacted/synthetic** where needed to avoid secrets.
- Expected status results assume **recent activity** unless otherwise noted.
