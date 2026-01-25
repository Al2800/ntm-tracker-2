# Performance Baseline (Spike)

This document captures initial measurements for tmux metadata polling (`tmux list-panes -a -F ...`) used by the daemon collector.

## Benchmark Script

Use `benchmarks/tmux_poll.sh`.

What it does:
- Creates an isolated tmux server (via `tmux -L <socket>`)
- Creates `SESSION_COUNT` sessions with `PANES_PER_SESSION` panes each
- Runs `tmux list-panes -a -F <format>` `RUNS` times and reports min/p50/p95/p99/max/avg wall-time (ms)
- Optionally measures CPU cost of 1Hz polling for `CPU_DURATION_SECONDS` seconds

Example:

```bash
# p50/p95/p99 latency for 20 sessions x 6 panes
bash benchmarks/tmux_poll.sh

# Also measure CPU cost of 1Hz polling for 5 minutes
MEASURE_CPU=1 CPU_DURATION_SECONDS=300 bash benchmarks/tmux_poll.sh
```

## Baseline Results (this dev container)

Environment:
- tmux: 3.4
- Shape: 20 sessions × 6 panes (120 panes)
- `FORMAT_STRING`: `#{session_name}|#{window_index}|#{pane_index}|#{pane_id}|#{pane_active}|#{pane_dead}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}|#{pane_title}`

Latency (`RUNS=100`):
- min: 7.61ms
- p50: 9.13ms
- p95: 12.34ms
- p99: 15.00ms
- max: 20.22ms
- avg: 9.61ms

CPU (1Hz polling for 300s):
- cpu_seconds (user+sys): 1.67s
- cpu_pct_one_core: 0.56%

## Recommendation

- `tmux list-panes -a -F ...` is comfortably within the latency budget (p99 well under 200ms).
- For CPU overhead, 1Hz polling is close to the 0.5% target in this baseline; if WSL measurements are higher, consider:
  - Defaulting to a 2s interval while idle (halves the duty cycle)
  - Using a lighter `FORMAT_STRING` on the hot path and fetching expensive fields less frequently

## Profiling Runbook (Daemon + App)

1. **Poll loop baseline**
   - `bash benchmarks/tmux_poll.sh`
2. **Command latency**
   - `bash benchmarks/command_latency.sh`
3. **Load test (multi-session)**
   - `tests/load/scale_test.sh` (see `docs/performance-results.md`)
4. **CPU flamegraph (daemon)**
   - `cargo install flamegraph` (once)
   - `sudo cargo flamegraph --bin ntm-tracker-daemon -- --stdio`
5. **Memory checks**
   - Use Task Manager / `Get-Process` on Windows
   - Record RSS before/after 60 minutes

## Bottlenecks to Watch

- Slow `ntm --robot-markdown` sections (avoid heavy sections with `--md-sections sessions`)
- Large `robot-tail` payloads (limit `--lines`)
- SQLite write stalls (check WAL size + busy timeouts)
- UI render spikes (large session lists without virtualization)

## Results (Fill In)

- Poll loop p95:
- Command latency p95:
- Event processing p95:
- Daemon memory delta (60m):
- UI render p95:
