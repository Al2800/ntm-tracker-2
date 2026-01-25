# NTM Tracker

**NTM Tracker** is a Windows system tray app that monitors NTM (Named Tmux Manager) sessions running in WSL2. It provides real‑time session/pane status, compact events, escalations, and usage analytics.

> **Project status**: Active development. Installer builds are in progress; some flows below describe the intended release experience.

## Features

- **Session & Pane Tracking** — live status (active/idle/waiting)
- **Compact Detection** — pattern matching + context‑size drops
- **Escalation Detection** — human input/error prompt detection
- **Usage Analytics** — hourly/daily aggregates, token estimates
- **System Tray Workflow** — quick status + per‑session actions
- **Dashboard UI** — overview cards, activity graph, escalation list

## Architecture

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
- `ntm` installed in WSL (Named Tmux Manager, optional but recommended)
- Node.js 20+ and Rust (for local builds)

## Installation

### Prebuilt (MSI)

1. Download the latest MSI from GitHub Releases (when published).
2. Run the installer and follow the first‑run wizard.
3. Launch the app; it will bootstrap the daemon in WSL.

### Build from source (Windows)

1. Install prerequisites: Node.js 20+, Rust stable, and Tauri build deps.
2. Install frontend deps:
   ```bash
   npm -C app install
   ```
3. Build the MSI:
   ```powershell
   ./scripts/build.ps1
   ```
4. Install the generated MSI from `app/src-tauri/target/release/bundle/msi/`.

## Quick Start

1. Install WSL2 + your distro.
2. Install `tmux` + `ntm` inside WSL.
3. Launch the Windows app (first run installs the daemon in WSL).
4. Confirm the tray icon shows live session status.
5. Open the dashboard to view sessions and activity.

## Configuration

See `docs/configuration.md` for daemon and app settings (poll intervals, notifications, quiet hours, and transport selection).

## Troubleshooting

See `docs/troubleshooting.md` for WSL connectivity tips, stdio fallback, and diagnostics guidance.

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
