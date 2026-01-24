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
- min: 8ms
- p50: 10ms
- p95: 13ms
- p99: 15ms
- max: 19ms
- avg: 10.06ms

CPU (1Hz polling for 300s):
- cpu_seconds (user+sys): 1.67s
- cpu_pct_one_core: 0.56%

## Recommendation

- `tmux list-panes -a -F ...` is comfortably within the latency budget (p99 well under 200ms).
- For CPU overhead, 1Hz polling is close to the 0.5% target in this baseline; if WSL measurements are higher, consider:
  - Defaulting to a 2s interval while idle (halves the duty cycle)
  - Using a lighter `FORMAT_STRING` on the hot path and fetching expensive fields less frequently
