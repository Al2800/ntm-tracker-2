# Command Timeout Configuration (Spike)

This document records baseline command-latency measurements and the resulting timeout recommendations.

## Measurement Harness

Use `benchmarks/command_latency.sh`.

Example:

```bash
# Idle system baseline
bash benchmarks/command_latency.sh

# Optional: measure under sustained CPU load
CPU_LOAD=1 bash benchmarks/command_latency.sh
```

The script reports min/p50/p95/p99/max/avg (ms) over `RUNS` executions.

## Baseline Results (this dev container)

Environment:
- tmux: 3.4
- Shape: 20 sessions × 6 panes (120 panes)

`tmux list-panes -a -F ...` (metadata polling hot path), `RUNS=100`:
- p99: 16.14ms
- max: 18.89ms

`ntm --robot-markdown --md-compact --md-sections sessions --md-session <session>`, `RUNS=100`:
- p99: 55.61ms
- max: 74.42ms

`ntm --robot-tail <session> --lines 50 --json`, `RUNS=100`:
- p99: 157.70ms
- max: 416.99ms

## Recommended Timeouts

These defaults aim to be conservative in the face of WSL variance, cold starts, and occasional system load:

- `tmux list-panes -a -F ...`: **3s timeout**
  - Rationale: baseline p99 is ~0.015s, so 3s is generous while still detecting hangs quickly.
- `ntm --robot-markdown` (status snapshot): **15s timeout** (pending WSL measurements)
  - Recommendation: pass `--md-sections sessions` to avoid slow sections (mail/beads/alerts) that can block on external dependencies.
- `ntm --robot-tail` (pane sampling): **20s timeout** (pending WSL measurements)

## Next Measurements Needed

To finalize defaults, re-run `benchmarks/command_latency.sh` inside WSL2 and (if possible) extend it to include the actual `ntm --robot-*` commands used by the daemon.
