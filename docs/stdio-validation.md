# Stdio JSON-RPC Validation Spike

This spike validates that stdio JSON-RPC supports bidirectional, real-time
notifications between Windows and WSL without relying on localhost networking.

## Harness Overview

The harness lives in `spikes/stdio_rpc/` and provides:

- A **daemon** mode that reads JSON-RPC from stdin and emits responses +
  notifications on stdout.
- A **client** mode that spawns the daemon process, sends ping requests,
  and measures latency + notification delivery.

The transport is line-delimited JSON (one JSON-RPC object per line).

## Build

```bash
cd spikes/stdio_rpc
cargo build --release
```

Binary: `spikes/stdio_rpc/target/release/stdio-rpc-spike`

## Local Smoke Test (Linux/WSL)

```bash
./target/release/stdio-rpc-spike client --pings 100 --rate-hz 100 --duration-secs 5
```

Expected output includes latency p50/p95/p99 and notification counts.

## Windows + WSL Execution

On Windows, spawn the daemon inside WSL via `wsl.exe`:

```powershell
# From repo root (PowerShell)
$daemon = "wsl.exe -d Ubuntu -- /home/<user>/ntracker3/spikes/stdio_rpc/target/release/stdio-rpc-spike daemon"
./spikes/stdio_rpc/target/release/stdio-rpc-spike client --pings 100 --rate-hz 100 --duration-secs 5 -- $daemon
```

Notes:
- Replace `Ubuntu` and path with your distro + repo location.
- Ensure the daemon writes **only** JSON lines to stdout.

## Measurement Checklist

Capture these metrics for each run:

- Round-trip latency (ms): p50 / p95 / p99
- Notification delivery: received vs expected
- Message loss rate (%)
- Any reconnect or shutdown anomalies

## Results (Fill In)

| Run | Environment | p50 | p95 | p99 | Notifications Received / Expected | Loss % | Notes |
| --- | ----------- | --- | --- | --- | -------------------------------- | ------ | ----- |
| 1   | Linux (local) | 18.44 ms | 32.19 ms | 33.20 ms | 300 / 300 | 0% | `cargo run -- client --pings 100 --rate-hz 100 --duration-secs 3` |
| 2   | Windows + WSL |     |     |     |                                  |        |       |

## Findings

- Summary: Local Linux run stays under 50ms p99 with no loss at 100 notif/sec.
- Failure modes observed: None in local run; Windows/WSL still pending.
- Recommendation on stdio default viability: Promising locally; confirm on Windows/WSL.

## Follow-Ups

- If p99 latency exceeds 50ms, investigate buffering or line framing.
- If loss occurs at 100 notif/sec, try 200–500 and note thresholds.
- Validate reconnect by terminating the daemon mid-run and restarting.
