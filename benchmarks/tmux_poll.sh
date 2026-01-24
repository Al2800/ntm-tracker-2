#!/usr/bin/env bash
set -euo pipefail

SOCKET_NAME="${SOCKET_NAME:-ntmtrackerbench_$$}"
SESSION_COUNT="${SESSION_COUNT:-20}"
PANES_PER_SESSION="${PANES_PER_SESSION:-6}"
RUNS="${RUNS:-100}"
MEASURE_CPU="${MEASURE_CPU:-0}"
CPU_DURATION_SECONDS="${CPU_DURATION_SECONDS:-300}"

DEFAULT_FORMAT_STRING='#{session_name}|#{window_index}|#{pane_index}|#{pane_id}|#{pane_active}|#{pane_dead}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}|#{pane_title}'
FORMAT_STRING="${FORMAT_STRING:-$DEFAULT_FORMAT_STRING}"

cleanup() {
  tmux -L "$SOCKET_NAME" kill-server >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "tmux: $(tmux -V)"
echo "socket: $SOCKET_NAME"
echo "sessions: $SESSION_COUNT"
echo "panes_per_session: $PANES_PER_SESSION"
echo "runs: $RUNS"
echo

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

total_panes_expected=$((SESSION_COUNT * PANES_PER_SESSION))
total_panes_actual="$(tmux -L "$SOCKET_NAME" list-panes -a | wc -l | tr -d ' ')"
echo "created panes: ${total_panes_actual}/${total_panes_expected}"
echo

echo "warming up..."
tmux -L "$SOCKET_NAME" list-panes -a -F "$FORMAT_STRING" >/dev/null

echo "running list-panes benchmark..."
durations_ms=()
for _ in $(seq 1 "$RUNS"); do
  start_ns="$(date +%s%N)"
  tmux -L "$SOCKET_NAME" list-panes -a -F "$FORMAT_STRING" >/dev/null
  end_ns="$(date +%s%N)"
  durations_ms+=( $(((end_ns - start_ns) / 1000000)) )
done

printf "%s\n" "${durations_ms[@]}" | sort -n | awk -v n="$RUNS" '
  { a[NR] = $1; sum += $1 }
  END {
    if (n < 1) { print "no samples"; exit 1 }

    p50i = int(0.50 * n + 0.5); if (p50i < 1) p50i = 1; if (p50i > n) p50i = n
    p95i = int(0.95 * n + 0.5); if (p95i < 1) p95i = 1; if (p95i > n) p95i = n
    p99i = int(0.99 * n + 0.5); if (p99i < 1) p99i = 1; if (p99i > n) p99i = n

    avg = sum / n
    min = a[1]
    max = a[n]

    print "results_ms:"
    print "  min:", min
    print "  p50:", a[p50i]
    print "  p95:", a[p95i]
    print "  p99:", a[p99i]
    print "  max:", max
    printf "  avg: %.2f\n", avg
  }
'

if [[ "$MEASURE_CPU" == "1" ]]; then
  echo
  echo "measuring CPU cost (1Hz polling for ${CPU_DURATION_SECONDS}s)..."

  cpu_time_output="$(
    {
      /usr/bin/time -p bash -c "
        set -euo pipefail
        for _ in \$(seq 1 \"$CPU_DURATION_SECONDS\"); do
          tmux -L \"$SOCKET_NAME\" list-panes -a -F \"$FORMAT_STRING\" >/dev/null
          sleep 1
        done
      " >/dev/null
    } 2>&1
  )"

  user_s="$(printf "%s\n" "$cpu_time_output" | awk '/^user /{print $2}')"
  sys_s="$(printf "%s\n" "$cpu_time_output" | awk '/^sys /{print $2}')"

  if [[ -n "${user_s:-}" && -n "${sys_s:-}" ]]; then
    cpu_total_s="$(awk -v u="$user_s" -v s="$sys_s" 'BEGIN{printf "%.6f", u + s}')"
    cpu_pct_one_core="$(awk -v t="$cpu_total_s" -v d="$CPU_DURATION_SECONDS" 'BEGIN{printf "%.2f", (t / d) * 100.0}')"
    echo "cpu_seconds (user+sys): $cpu_total_s"
    echo "cpu_pct_one_core: ${cpu_pct_one_core}%"
  else
    echo "cpu measurement unavailable (expected /usr/bin/time output)."
    echo "$cpu_time_output"
  fi
fi
