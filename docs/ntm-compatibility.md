# NTM Compatibility Notes (Robot Outputs)

This project relies on NTM (Named Tmux Manager) “robot” outputs for session/pane status and (optionally) pane output sampling.

## Version Observed

Fixtures were captured from:

- `ntm version`: v1.2.0 (linux/amd64), commit `d78257aa8d5077fee2f62a5ff3278e38fdec5732`

Note: `ntm --version` is **not** supported in this build; use `ntm version` or `ntm --robot-version`.

## Captured Output Formats

See `fixtures/ntm/README.md`.

Key files:
- `fixtures/ntm/robot-status.json` (`ntm --robot-status`)
- `fixtures/ntm/robot-markdown.md` (`ntm --robot-markdown`)
- `fixtures/ntm/robot-markdown-compact.md` (`ntm --robot-markdown --md-compact`)
- `fixtures/ntm/robot-tail.json` (`ntm --robot-tail <session> --lines 50 --json`)
- `fixtures/ntm/robot-terse.txt` (`ntm --robot-terse`)

## Important Observations

- `ntm --robot-status` provides a JSON structure with `sessions[]` including `name`, `windows`, `panes`, and `agents[]` (agent `type`, pane/window indices, and active flag). This is sufficient to enumerate sessions/panes and basic agent-type attribution.
- `ntm --robot-markdown` can be made fast and deterministic by limiting sections:
  - Use `--md-sections sessions` to avoid slow sections that may depend on external services (e.g., Agent Mail).
- `ntm --robot-tail` returns JSON containing `panes` keyed by pane index with `lines[]` (pane output), `state`, `type`, and `truncated`. This enables output-driven detectors (compact/escalation) when enabled.

## Missing / Unclear Fields

The outputs captured above do not include explicit:
- Compact counts per pane/session
- Context sizes / token estimates

`ntm --robot-terse` includes a `C:<number>%` field per session, but its meaning should be confirmed upstream (it may be “context fullness” or a compact-related metric).

### Workaround Plan

- Treat “compact events” as an output-derived detector:
  - Tail recent pane output (`--robot-tail`) and use regex/pattern-based detection for known “context compacted” markers.
- Treat “token estimates” as best-effort:
  - Derive from client-side heuristics (e.g., approximate token count from output length) or integrate with whichever agent CLI exposes reliable token usage.

## Known Version Differences

Only NTM v1.2.0 is validated in this repository at the moment.

If behavior differs on other versions, add additional captures under `fixtures/ntm/` (e.g., `robot-status.v1.1.x.json`) and update this doc with deltas.
