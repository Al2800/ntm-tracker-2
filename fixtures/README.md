# NTM Tracker Fixtures

Test fixtures for parser and detector golden tests.

## Directory Structure

```
fixtures/
├── README.md           # This file
├── ntm/                # NTM robot output format samples
│   ├── robot-status.json
│   ├── robot-markdown.md
│   ├── robot-tail.json
│   └── ...
├── sessions/           # Pane output samples for detection tests
│   ├── README.md
│   ├── cuas-sim-claude.json      # Real Claude session
│   ├── ntracker3-codex.json      # Real Codex session
│   ├── compact-trigger-sample.json    # Synthetic compact detection
│   ├── escalation-sample.json         # Synthetic escalation detection
│   └── ...
└── expected/           # Expected parse results
    ├── compact-trigger-expected.json
    ├── escalation-expected.json
    └── status-transitions-expected.json
```

## Fixture Types

### NTM Robot Outputs (`ntm/`)
Samples of NTM `--robot-*` command outputs for validating:
- Output format parsing
- Field extraction
- Version compatibility

See `ntm/README.md` for details.

### Session Captures (`sessions/`)
Pane output samples for testing:
- Agent type detection
- Status detection (active/idle/waiting)
- Compact event detection
- Escalation detection
- ANSI escape code handling
- Unicode handling

See `sessions/README.md` for details.

### Expected Results (`expected/`)
Expected parse results for golden tests. Each file corresponds to a session fixture and defines what the parser/detector should extract.

## Coverage Summary

| Category | Real Samples | Synthetic | Total |
|----------|--------------|-----------|-------|
| Claude sessions | 1 | 1 | 2 |
| Codex sessions | 1 | 0 | 1 |
| Mixed/shell panes | 2 | 2 | 4 |
| Compact detection | 0 | 1 | 1 |
| Escalation detection | 0 | 1 | 1 |
| ANSI handling | 0 | 1 | 1 |
| Unicode handling | 0 | 1 | 1 |
| **Total session fixtures** | **4** | **7** | **11** |

## Usage

### Rust Tests
```rust
use std::fs;
use serde_json::Value;

fn load_fixture(path: &str) -> Value {
    let content = fs::read_to_string(format!("fixtures/{}", path))
        .expect("Failed to read fixture");
    serde_json::from_str(&content).expect("Failed to parse fixture")
}

#[test]
fn test_compact_detection() {
    let session = load_fixture("sessions/compact-trigger-sample.json");
    let expected = load_fixture("expected/compact-trigger-expected.json");

    let result = detect_compacts(&session);
    assert_eq!(
        result.count,
        expected["expected_compact_count"].as_u64().unwrap()
    );
}
```

## Sensitive Data

All fixtures are reviewed before commit:
- No API keys, tokens, or credentials
- No personal information
- Paths sanitized where practical
- Real sessions captured from controlled test environments

## Maintenance

When adding new fixtures:
1. Create session file in `sessions/`
2. Create expected result in `expected/` (if applicable)
3. Update `sessions/README.md` with description
4. Update coverage table above
5. Verify no sensitive data
