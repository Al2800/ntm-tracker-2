#!/usr/bin/env bash
set -euo pipefail

RUNS="${RUNS:-100}"
SOCKET_NAME="${SOCKET_NAME:-ntmtrackerlat_$$}"
SETUP_TMUX="${SETUP_TMUX:-1}"
SESSION_COUNT="${SESSION_COUNT:-20}"
PANES_PER_SESSION="${PANES_PER_SESSION:-6}"

CPU_LOAD="${CPU_LOAD:-0}"
LOAD_COMMAND="${LOAD_COMMAND:-yes > /dev/null}"

DEFAULT_TMUX_FORMAT_STRING='#{session_name}|#{window_index}|#{pane_index}|#{pane_id}|#{pane_active}|#{pane_dead}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}|#{pane_title}'
TMUX_FORMAT_STRING="${TMUX_FORMAT_STRING:-$DEFAULT_TMUX_FORMAT_STRING}"

NTM_MARKDOWN_SECTIONS="${NTM_MARKDOWN_SECTIONS:-sessions}"
MEASURE_NTM_FULL_MARKDOWN="${MEASURE_NTM_FULL_MARKDOWN:-0}"
NTM_TAIL_LINES="${NTM_TAIL_LINES:-50}"

cleanup() {
  if [[ "${load_pid:-}" != "" ]]; then
    kill "$load_pid" >/dev/null 2>&1 || true
  fi
  tmux -L "$SOCKET_NAME" kill-server >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "tmux: $(tmux -V)"
echo "runs: $RUNS"
echo "cpu_load: $CPU_LOAD"
echo

if [[ "$CPU_LOAD" == "1" ]]; then
  echo "starting background load: $LOAD_COMMAND"
  bash -c "$LOAD_COMMAND" &
  load_pid="$!"
  echo "load_pid: $load_pid"
  echo
fi

if [[ "$SETUP_TMUX" == "1" ]]; then
  echo "creating benchmark tmux server..."
  for s in $(seq 1 "$SESSION_COUNT"); do
    session="bench_${s}"
    tmux -L "$SOCKET_NAME" new-session -d -s "$session" -x 200 -y 50 "sleep 1000000"

    for p in $(seq 2 "$PANES_PER_SESSION"); do
      if (( p % 2 == 0 )); then
        tmux -L "$SOCKET_NAME" split-window -t "$session" -h "sleep 1000000"
      else
        tmux -L "$SOCKET_NAME" split-window -t "$session" -v "sleep 1000000"
      fi
      tmux -L "$SOCKET_NAME" select-layout -t "$session" tiled >/dev/null
    done
  done
  total_panes_actual="$(tmux -L "$SOCKET_NAME" list-panes -a | wc -l | tr -d ' ')"
  echo "created panes: $total_panes_actual"
  echo
fi

stats_from_ms() {
  local label="$1"
  shift
  printf "%s\n" "$@" | sort -n | awk -v label="$label" '
    { a[NR] = $1; sum += $1 }
    END {
      n = NR
      if (n < 1) { print label ": no samples"; exit 1 }

      p50i = int(0.50 * n + 0.5); if (p50i < 1) p50i = 1; if (p50i > n) p50i = n
      p95i = int(0.95 * n + 0.5); if (p95i < 1) p95i = 1; if (p95i > n) p95i = n
      p99i = int(0.99 * n + 0.5); if (p99i < 1) p99i = 1; if (p99i > n) p99i = n

      avg = sum / n
      min = a[1]
      max = a[n]

      print label ":"
      print "  runs:", n
      print "  min_ms:", min
      print "  p50_ms:", a[p50i]
      print "  p95_ms:", a[p95i]
      print "  p99_ms:", a[p99i]
      print "  max_ms:", max
      printf "  avg_ms: %.2f\n", avg
    }
  '
}

measure() {
  local label="$1"
  local cmd="$2"

  echo "measuring: $label"
  echo "  cmd: $cmd"

  LABEL="$label" CMD="$cmd" RUNS="$RUNS" python3 - <<'PY'
import os
import subprocess
import time

label = os.environ["LABEL"]
cmd = os.environ["CMD"]
runs = int(os.environ["RUNS"])

durations_ms = []
for _ in range(runs):
    start_ns = time.perf_counter_ns()
    subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    end_ns = time.perf_counter_ns()
    durations_ms.append((end_ns - start_ns) / 1_000_000.0)

durations_ms.sort()

def idx_for_percentile(p: float) -> int:
    # Match the shell script convention: int(p*n + 0.5) with 1-based indexing.
    i_1_based = int(p * runs + 0.5)
    if i_1_based < 1:
        i_1_based = 1
    if i_1_based > runs:
        i_1_based = runs
    return i_1_based - 1

min_ms = durations_ms[0]
max_ms = durations_ms[-1]
avg_ms = sum(durations_ms) / runs
p50_ms = durations_ms[idx_for_percentile(0.50)]
p95_ms = durations_ms[idx_for_percentile(0.95)]
p99_ms = durations_ms[idx_for_percentile(0.99)]

print()
print(f"{label}:")
print(f"  runs: {runs}")
print(f"  min_ms: {min_ms:.2f}")
print(f"  p50_ms: {p50_ms:.2f}")
print(f"  p95_ms: {p95_ms:.2f}")
print(f"  p99_ms: {p99_ms:.2f}")
print(f"  max_ms: {max_ms:.2f}")
print(f"  avg_ms: {avg_ms:.2f}")
print()
PY
}

measure "tmux list-panes -a -F (metadata polling hot path)" \
  "tmux -L \"$SOCKET_NAME\" list-panes -a -F \"$TMUX_FORMAT_STRING\""

if command -v ntm >/dev/null 2>&1; then
  echo "ntm detected: $(ntm version 2>/dev/null | head -n 1 || true)"

  ntm_session="${NTM_SESSION:-}"
  if [[ "$ntm_session" == "" ]]; then
    ntm_session="$(ntm --robot-status 2>/dev/null | jq -r '.sessions[0].name // empty' || true)"
  fi

  if [[ "$ntm_session" == "" ]]; then
    echo "no ntm-managed sessions found; skipping ntm latency measurements."
    echo "hint: create/spawn a session via ntm, then re-run with NTM_SESSION=<name>."
    echo
  else
    echo "ntm session for measurements: $ntm_session"
    echo

    measure "ntm --robot-markdown --md-compact --md-sections=${NTM_MARKDOWN_SECTIONS} --md-session=<session>" \
      "ntm --robot-markdown --md-compact --md-sections \"$NTM_MARKDOWN_SECTIONS\" --md-session \"$ntm_session\""

    if [[ "$MEASURE_NTM_FULL_MARKDOWN" == "1" ]]; then
      measure "ntm --robot-markdown --md-compact (full/default sections)" \
        "ntm --robot-markdown --md-compact --md-session \"$ntm_session\""
    fi

    measure "ntm --robot-tail <session> --lines=${NTM_TAIL_LINES} --json" \
      "ntm --robot-tail \"$ntm_session\" --lines \"$NTM_TAIL_LINES\" --json"
  fi
else
  echo "ntm not found; skipping ntm latency measurements."
  echo
fi
