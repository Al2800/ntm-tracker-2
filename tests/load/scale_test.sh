#!/usr/bin/env bash
set -euo pipefail

SESSION_PREFIX="${SESSION_PREFIX:-ntm-load}"
SESSION_COUNT="${SESSION_COUNT:-20}"
PANE_COUNT="${PANE_COUNT:-6}"
RUN_MINUTES="${RUN_MINUTES:-30}"
ACTIVITY_INTERVAL_SEC="${ACTIVITY_INTERVAL_SEC:-2}"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required to run this load test."
  exit 1
fi

cleanup() {
  tmux list-sessions -F '#S' 2>/dev/null | grep "^${SESSION_PREFIX}-" | while read -r sess; do
    tmux kill-session -t "$sess" || true
  done
}

trap cleanup EXIT

echo "Creating ${SESSION_COUNT} tmux sessions with ${PANE_COUNT} panes each"

for i in $(seq 1 "$SESSION_COUNT"); do
  session="${SESSION_PREFIX}-${i}"
  tmux new-session -d -s "$session" "bash"
  for _ in $(seq 2 "$PANE_COUNT"); do
    tmux split-window -t "$session" -h "bash"
    tmux select-layout -t "$session" tiled >/dev/null
  done
done

echo "Starting activity loops in each pane"
for i in $(seq 1 "$SESSION_COUNT"); do
  session="${SESSION_PREFIX}-${i}"
  pane_ids=$(tmux list-panes -t "$session" -F '#{pane_index}')
  for pane_id in $pane_ids; do
    tmux send-keys -t "${session}:0.${pane_id}" \
      "while true; do date +%s%3N; sleep ${ACTIVITY_INTERVAL_SEC}; done" C-m
  done
done

echo "Load running for ${RUN_MINUTES} minutes. Press Ctrl-C to stop early."
sleep "$((RUN_MINUTES * 60))"

echo "Load test complete; cleaning up sessions"
