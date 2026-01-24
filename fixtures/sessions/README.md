# Session Fixtures

Real and synthetic session samples for parser and detector golden tests.

## Real Sessions (Captured from Live NTM)

| File | Session | Agent Types | Description |
|------|---------|-------------|-------------|
| `cuas-sim-claude.json` | cuas-sim | Claude | Claude Code agents working on project |
| `ntracker3-codex.json` | ntracker3 | Codex | Codex agents working on NTM Tracker |
| `speedread-ios-mixed.json` | speedread-ios | Mixed | Mixed session with various pane types |
| `status-snapshot.json` | all | n/a | Full system status snapshot |

## Synthetic Samples (For Specific Test Cases)

| File | Purpose | Key Patterns |
|------|---------|--------------|
| `compact-trigger-sample.json` | Compact detection testing | Context % warnings, auto-compact trigger |
| `escalation-sample.json` | Escalation detection testing | User choice prompts, permission errors |
| `status-transitions-sample.json` | Status detection testing | active/idle/waiting states |
| `ansi-heavy-sample.json` | ANSI escape code handling | Color codes, formatting sequences |
| `unicode-sample.json` | Unicode handling | Emoji, CJK characters, RTL text |
| `shell-only-sample.json` | Non-agent pane detection | Pure shell without AI agent |

## File Format

All session files follow the `ntm --robot-tail` output format:

```json
{
  "session": "session-name",
  "captured_at": "ISO8601 timestamp",
  "panes": {
    "0": {
      "type": "claude|codex|gemini|unknown",
      "state": "active|idle|waiting",
      "lines": ["line1", "line2", ...],
      "truncated": true|false
    }
  }
}
```

Synthetic samples may include additional `description` field.

## Usage in Tests

```rust
#[test]
fn test_compact_detection() {
    let fixture = load_fixture("sessions/compact-trigger-sample.json");
    let expected = load_fixture("expected/compact-trigger-expected.json");

    let result = parse_session(&fixture);
    assert_eq!(result.compact_count, expected.expected_compact_count);
}
```

## Sensitive Data Policy

- Real session captures are reviewed for sensitive data before commit
- API keys, tokens, and credentials are NEVER included
- Paths are sanitized to generic forms where practical
- Personal information is redacted

## Regenerating Real Fixtures

```bash
ntm --robot-tail <session> --lines 100 > fixtures/sessions/<name>.json
ntm --robot-status > fixtures/sessions/status-snapshot.json
```

## Adding New Fixtures

1. Capture or create the session file
2. Add corresponding expected result in `../expected/`
3. Update this README with the new file
4. Verify no sensitive data is included
