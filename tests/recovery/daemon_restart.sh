#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DAEMON_BIN="${DAEMON_BIN:-$ROOT_DIR/daemon/target/release/ntm-tracker-daemon}"
PORT="${PORT:-9847}"
CONFIG_PATH="${CONFIG_PATH:-$ROOT_DIR/tmp/recovery/daemon.toml}"
LOG_DIR="${LOG_DIR:-$ROOT_DIR/tmp/recovery}"

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

start_daemon() {
  "$DAEMON_BIN" start --http-port "$PORT" --config "$CONFIG_PATH" >/dev/null 2>&1 &
  echo $!
}

wait_for_health() {
  for _ in $(seq 1 10); do
    if "$DAEMON_BIN" health --port "$PORT" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "Health check failed after retries"
  return 1
}

cleanup() {
  "$DAEMON_BIN" stop >/dev/null 2>&1 || true
}

trap cleanup EXIT

echo "Scenario 1: clean restart"
pid=$(start_daemon)
wait_for_health
cleanup
wait "$pid" 2>/dev/null || true
pid=$(start_daemon)
wait_for_health
cleanup
wait "$pid" 2>/dev/null || true

echo "Scenario 2: crash recovery"
pid=$(start_daemon)
wait_for_health
kill -9 "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
pid=$(start_daemon)
wait_for_health
cleanup
wait "$pid" 2>/dev/null || true

echo "Daemon restart recovery checks completed"
