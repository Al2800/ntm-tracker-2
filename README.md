# NTM Tracker

**NTM Tracker** is a Windows system tray app that monitors NTM (Named Tmux Manager) sessions running in WSL2. It provides real‑time session/pane status, compact events, escalations, and usage analytics.

> **Project status**: Early stage. This repository currently contains specs, fixtures, and spikes. Documentation below reflects the **intended** behavior described in `PLAN.md`.

## Features (Planned)

- **Session & Pane Tracking** — live status (active/idle/waiting)
- **Compact Detection** — pattern matching + context‑size drops
- **Escalation Detection** — human input/error prompt detection
- **Usage Analytics** — hourly/daily aggregates, token estimates
- **System Tray Workflow** — quick status + per‑session actions
- **Dashboard UI** — overview cards, activity graph, escalation list

## Architecture (Planned)

```
Presentation  → Svelte UI (Dashboard + Tray)
Backend       → Tauri (Rust) app shell + daemon manager
Service       → WSL daemon (Rust)
Data          → SQLite + in‑memory cache
```

Default transport is **stdio JSON‑RPC via `wsl.exe`**, with optional WebSocket/HTTP for service mode.

## Screenshot (Placeholder)

```
┌──────────────────────────────────────────────────────────────┐
│ NTM Tracker                                                   │
│  Sessions  |  Activity  |  Escalations                        │
│  - api:0   | ███░░░░░░░ | ⚠️ 1 pending                         │
│  - research| █████░░░░░ |                                     │
└──────────────────────────────────────────────────────────────┘
```

## Requirements

- Windows 10/11 with WSL2
- A WSL distro (Ubuntu recommended)
- `tmux` installed in WSL
- `ntm` installed in WSL (Named Tmux Manager)

## Quick Start (Planned Flow)

1. Install WSL2 + your distro.
2. Install `tmux` + `ntm` in WSL.
3. Launch the Windows app (first run installs the daemon in WSL).
4. Confirm the tray icon shows live session status.

## Docs

- Installation: `docs/install.md`
- Configuration: `docs/configuration.md`
- Troubleshooting: `docs/troubleshooting.md`
- Architecture overview: `docs/architecture.md`
- Demo capture guide: `docs/demo.md`
- Release checklist: `docs/release-checklist.md`
- Full technical spec: `PLAN.md`
- Changelog: `CHANGELOG.md`

## Contributing

Issue tracking uses **Beads** (`bd`). See `AGENTS.md` for workflow and constraints.

### Golden Tests

Snapshot tests for parsers and detectors live in `daemon/tests/golden/` and use `insta`.
From `daemon/`, run `cargo test` to execute them. To accept intentional changes, run
`cargo insta accept` or re-run with `INSTA_UPDATE=always`.
