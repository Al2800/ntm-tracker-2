#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DAEMON_BIN="${DAEMON_BIN:-$ROOT_DIR/daemon/target/release/ntm-tracker-daemon}"
ITERATIONS="${ITERATIONS:-100}"
SLEEP_MAX_MS="${SLEEP_MAX_MS:-750}"
SLEEP_MIN_MS="${SLEEP_MIN_MS:-150}"

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/ntm-tracker"
DB_PATH="${DB_PATH:-$DATA_DIR/ntm-tracker.db}"
LOG_DIR="${LOG_DIR:-$ROOT_DIR/tmp/chaos}"
CONFIG_PATH="${CONFIG_PATH:-$LOG_DIR/daemon.toml}"

mkdir -p "$LOG_DIR"

if [[ ! -x "$DAEMON_BIN" ]]; then
  echo "Daemon binary not found: $DAEMON_BIN"
  echo "Build it first: cargo build -p ntm-tracker-daemon --release"
  exit 1
fi

cat > "$CONFIG_PATH" <<EOF
[logging]
level = "info"
file = "$LOG_DIR/daemon.log"
EOF

if [[ ! -f "$DB_PATH" ]]; then
  echo "Database not found at $DB_PATH."
  echo "Start the daemon once to initialize the DB before running chaos."
  exit 1
fi

echo "Running SQLite WAL chaos test for $ITERATIONS iterations"
echo "DB: $DB_PATH"
echo "Logs: $LOG_DIR"

for i in $(seq 1 "$ITERATIONS"); do
  echo "Iteration $i/$ITERATIONS"
  "$DAEMON_BIN" start --config "$CONFIG_PATH" >/dev/null 2>&1 &
  DAEMON_PID=$!

  sleep_ms=$((SLEEP_MIN_MS + RANDOM % (SLEEP_MAX_MS - SLEEP_MIN_MS + 1)))
  sleep_sec=$(awk "BEGIN {print $sleep_ms/1000}")
  sleep "$sleep_sec"

  if kill -0 "$DAEMON_PID" 2>/dev/null; then
    kill -9 "$DAEMON_PID" 2>/dev/null || true
  fi

  wait "$DAEMON_PID" 2>/dev/null || true

  if ! sqlite3 "$DB_PATH" "PRAGMA integrity_check;" | grep -q "ok"; then
    echo "Integrity check failed on iteration $i"
    exit 2
  fi
done

echo "Chaos test complete"
