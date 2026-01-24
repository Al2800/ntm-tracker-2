# Parser Fuzz Testing

Fuzz tests for NTM Tracker daemon parsers using cargo-fuzz with libfuzzer backend.

## Prerequisites

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz

# Or use rustup to install nightly
rustup install nightly
```

## Fuzz Targets

| Target | Parser | Description |
|--------|--------|-------------|
| `fuzz_ntm_markdown` | `parsers::ntm_markdown` | NTM `--robot-markdown` output |
| `fuzz_ntm_tail` | `parsers::ntm_tail` | NTM `--robot-tail` JSON output |
| `fuzz_tmux_panes` | `parsers::tmux_panes` | tmux `list-panes` format |
| `fuzz_compact_detector` | `detector::compact` | Compact event detection |

## Running Fuzz Tests

### Run a specific target
```bash
cd daemon
cargo +nightly fuzz run fuzz_ntm_markdown -- -max_total_time=60
```

### Run all targets (60s each)
```bash
cd daemon
for target in fuzz_ntm_markdown fuzz_ntm_tail fuzz_tmux_panes fuzz_compact_detector; do
    cargo +nightly fuzz run $target -- -max_total_time=60
done
```

### Run with specific iteration count
```bash
cargo +nightly fuzz run fuzz_ntm_markdown -- -runs=10000
```

## Corpus

Seed corpus files are in `corpus/<target>/`. These are seeded from fixtures in `fixtures/`.

To add more corpus entries:
```bash
cp /path/to/sample daemon/fuzz/corpus/fuzz_ntm_markdown/
```

## Crashes

If fuzzing finds a crash:
1. Crash inputs are saved in `artifacts/<target>/`
2. Reproduce with: `cargo +nightly fuzz run <target> artifacts/<target>/crash-*`
3. Fix the bug in the parser
4. Re-run to verify fix

## CI Integration

See `.github/workflows/fuzz.yml` for CI configuration.

CI runs each target for 60 seconds on pushes to main and PRs.

## Success Criteria

- All targets run 10k+ iterations without panic
- Any crashes are fixed before Phase 1
- Corpus grows over time as new edge cases are discovered
