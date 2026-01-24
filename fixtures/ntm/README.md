# NTM Robot Output Fixtures

Captured from NTM v1.2.0 on 2026-01-24.

## Files

| File | Description | Command |
|------|-------------|---------|
| `robot-status.json` | Full system status | `ntm --robot-status` |
| `robot-markdown.md` | Markdown summary table | `ntm --robot-markdown` |
| `robot-markdown-compact.md` | Ultra-compact markdown | `ntm --robot-markdown --md-compact` |
| `robot-tail.json` | Pane output capture | `ntm --robot-tail <session> --lines 50` |
| `robot-snapshot.json` | Unified system state | `ntm --robot-snapshot` |
| `robot-terse.txt` | Single-line terse status | `ntm --robot-terse` |

## Usage

These fixtures are used for:
1. Parser golden tests - validate parsing logic against known output
2. Detector tests - ensure compact/escalation patterns are detected
3. Documentation - reference examples for output formats

## Regenerating

```bash
ntm --robot-status > fixtures/ntm/robot-status.json
ntm --robot-markdown > fixtures/ntm/robot-markdown.md
ntm --robot-markdown --md-compact > fixtures/ntm/robot-markdown-compact.md
ntm --robot-tail ntracker3 --lines 50 > fixtures/ntm/robot-tail.json
ntm --robot-snapshot > fixtures/ntm/robot-snapshot.json
ntm --robot-terse > fixtures/ntm/robot-terse.txt
```

## Sensitive Data

These fixtures were captured from live sessions. Check for:
- API keys (should not be present)
- Sensitive paths (redacted as needed)
- Personal information

All current fixtures have been reviewed and contain no sensitive data.
