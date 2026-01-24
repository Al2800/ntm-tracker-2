# NTM Tracker - Detailed Project Plan

**Goal**: A Windows system tray application that monitors NTM (Named Tmux Manager) sessions running in WSL2, displaying real-time metrics including session status, pane activity, compact events, and usage statistics.

---

## 1. System Architecture

### 1.1 Component Overview

```
┌────────────────────────────────────────────────────────────────────────┐
│ WINDOWS HOST                                                           │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ NTM Tracker UI (Tauri App)                                       │ │
│  │                                                                  │ │
│  │  ┌─────────────┐  ┌─────────────────────────────────────────┐   │ │
│  │  │ System Tray │  │ Dashboard Window                        │   │ │
│  │  │             │  │                                         │   │ │
│  │  │ • Icon      │  │ • Session list with live status         │   │ │
│  │  │ • Badge     │  │ • Pane details (compacts, tokens, time) │   │ │
│  │  │ • Tooltip   │  │ • Usage graphs (hourly/daily)           │   │ │
│  │  │ • Menu      │  │ • Escalation alerts                     │   │ │
│  │  └─────────────┘  └─────────────────────────────────────────┘   │ │
│  │                                                                  │ │
│  │  Tauri Backend (Rust)                                           │ │
│  │  • Connection manager (stdio first; WS/HTTP optional)           │ │
│  │  • Daemon bootstrapper (wsl.exe; systemd optional)              │ │
│  │  • Windows notification integration                             │ │
│  │  • Auto-start on login                                          │ │
│  └───────────────────────────────┬──────────────────────────────────┘ │
│                                  │                                     │
│                                  │ Default: stdio JSON-RPC via wsl.exe    │
│                                  │ Optional: WS/HTTP (localhost)          │
│                                  │                                     │
└──────────────────────────────────┼─────────────────────────────────────┘
                                   │
┌──────────────────────────────────┼─────────────────────────────────────┐
│ WSL2 (Ubuntu)                    │                                     │
│                                  │                                     │
│  ┌───────────────────────────────▼──────────────────────────────────┐ │
│  │ ntm-tracker-daemon (Rust)                                        │ │
│  │                                                                  │ │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │ │
│  │  │ RPC Transports  │  │ Event Bus       │  │ Metrics Engine  │  │ │
│  │  │ stdio (default) │  │ WS/stdio deltas │  │                 │  │ │
│  │  │ WS/HTTP (opt)   │  │ Subscriptions   │  │ • Poll budgets  │  │ │
│  │  │ JSON-RPC 2.0    │  │                 │  │ • Aggregation   │  │ │
│  │  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │ │
│  │           │                    │                    │           │ │
│  │           └────────────────────┼────────────────────┘           │ │
│  │                                │                                 │ │
│  │                    ┌───────────▼───────────┐                    │ │
│  │                    │ Data Layer            │                    │ │
│  │                    │                       │                    │ │
│  │                    │ • SQLite database     │                    │ │
│  │                    │ • In-memory cache     │                    │ │
│  │                    └───────────┬───────────┘                    │ │
│  │                                │                                 │ │
│  └────────────────────────────────┼─────────────────────────────────┘ │
│                                   │                                    │
│  ┌────────────────────────────────▼─────────────────────────────────┐ │
│  │ NTM / Tmux                                                       │ │
│  │                                                                  │ │
│  │  ntm --robot-markdown    ntm --robot-tail    tmux capture/pipe  │ │
│  │                                                                  │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Technology Choices

| Component | Technology | Rationale |
|-----------|------------|-----------|
| WSL Daemon | **Rust** (tokio) for release; optional **Bun/Node** dev harness | Deterministic install (single binary), lower runtime variance, easier upgrades and better perf; keep TS harness for fixtures and fast iteration |
| Database | **SQLite** (rusqlite or sqlx) | Zero config, single file; stable bindings and good WAL performance |
| Windows App | **Tauri 2.0** | Small bundle (~5MB), native system tray, Rust perf |
| UI Framework | **Svelte 5** | Lightweight, reactive, pairs well with Tauri |
| IPC | **Single canonical JSON-RPC 2.0 protocol**; transports: **stdio** (default), **WebSocket**, optional **HTTP POST /rpc** | One schema, one handler surface; transports become adapters, reducing divergence and test burden |

### 1.3 Data Flow

```
1. Fast tmux metadata poll (every 1–2 seconds; cheap and stable):
   ┌─────────┐     ┌──────────────────┐     ┌─────────┐
   │ Daemon  │────▶│ tmux list-panes -a │──▶│ Diff +  │
   │ timer   │     │ (metadata only)  │     │ state   │
   └─────────┘     └──────────────────┘     └─────────┘

2. Structured NTM reconcile (adaptive 10–60 seconds; activity-aware):
   ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
   │ Daemon  │────▶│ NTM CLI │────▶│ Parser  │────▶│ SQLite  │
   │ timer   │     │ commands│     │         │     │ + cache │
   └─────────┘     └─────────┘     └─────────┘     └─────────┘

3. Pane Output (default: off; optional on-demand/stream):
   ┌─────────┐     ┌──────────────┐     ┌─────────┐     ┌─────────┐
   │ Daemon  │────▶│ capture-pane │────▶│ Redactor│────▶│ UI/API   │
   │ manager │     │ (on demand)  │     │         │     │ preview  │
   └─────────┘     └──────────────┘     └─────────┘     └─────────┘

   Optional stream mode (explicit opt-in; bounded):
   tmux pipe-pane → FIFO/rotated logs → tailer → detectors

4. Client Update (event bus first; DB is the sink):
   ┌──────────┐     ┌──────────────────────────────┐     ┌─────────┐
   │ Diff +   │────▶│ Push deltas (active transport)│────▶│ Tauri   │
   │ state    │     │ stdio (default) or WS         │     │ UI      │
   └──────────┘     └──────────────────────────────┘     └─────────┘

   HTTP-only clients fall back to cursor polling via `events.list` + `sessions.list`.
```

### 1.4 Operating Modes & Transport Selection

The plan currently treats WebSocket as the primary real-time path, which reintroduces the exact failure mode stdio is meant to avoid. Make stdio capable of the full real-time experience (JSON-RPC requests plus JSON-RPC notifications) and treat WS and HTTP as optional service-mode transports.

**Operating modes:**
- **Mode A (default): App-supervised stdio**
  - Windows app starts the daemon via `wsl.exe` and keeps a single long-lived stdio session open.
  - Real-time updates are delivered as JSON-RPC notifications on the same framed stdio stream.
  - No listening port is required; this is the most reliable mode on machines with brittle WSL localhost forwarding.

- **Mode B (optional): WSL service with WS or HTTP**
  - Daemon runs under systemd (when available) or as a long-lived background process.
  - Windows app connects over WebSocket for push deltas; HTTP is reserved for diagnostics and read-only polling clients.

- **Mode C (headless): CLI-only**
  - `ntm-tracker` CLI talks to the daemon using the same JSON-RPC protocol (stdio or WS), useful for scripting and support.

**Transport selection policy:**
1. Prefer stdio for default installs and for any environment where WSL networking is flaky.
2. Prefer WS only when a stable loopback path is confirmed and multi-client access is required.
3. Treat HTTP as optional and read-mostly; it cannot deliver true server push without long-polling.

### 1.5 Command Execution, Budgets, and Timeouts

Most failure modes in this system will be caused by spawning external processes (`tmux`, `ntm`, and sometimes `wsl.exe`) and assuming they are fast, well-behaved, and always available. Treat command execution as a first-class subsystem.

**CommandRunner requirements:**
- **Hard timeouts** per command category; kill the process group on timeout.
- **Output caps** to prevent pathological stdout growth from consuming memory.
- **Concurrency limits** and jitter to prevent spawn storms on large tmux servers.
- **Circuit breaker**: after N consecutive failures, back off aggressively and surface `degraded` health without mutating session state (avoid falsely marking sessions as ended).
- **Coalesce polls**: prefer one `tmux list-panes -a -F ...` per cycle rather than per session.
- **Explicit error taxonomy**: distinguish "tmux server unavailable" from "no sessions exist".

**SQLite write path:**
- Use a single writer task (actor) or a blocking thread pool boundary; avoid blocking the tokio runtime with synchronous sqlite calls.
- Batch inserts for `pane_minute_samples` and `events` in transactions to reduce fsync pressure.

---

## 2. Data Models

### 2.1 SQLite Schema

```sql
-- SQLite settings (applied on startup / connection)
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;

-- Metadata and daemon run tracking (diagnostics, upgrades, support)
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE daemon_runs (
    run_id TEXT PRIMARY KEY,        -- uuidv7
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    version TEXT NOT NULL,
    protocol_version INTEGER NOT NULL,
    schema_version INTEGER NOT NULL,
    capabilities TEXT               -- JSON
);

-- Distinguish multiple tmux servers and/or WSL distros (future-proofing)
CREATE TABLE sources (
    source_id TEXT PRIMARY KEY,     -- uuidv7
    kind TEXT NOT NULL,             -- 'wsl' (future: 'ssh', 'remote')
    distro TEXT NOT NULL,           -- WSL distro name (e.g. 'Ubuntu')
    tmux_socket TEXT,               -- optional; null uses default tmux server
    created_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    status TEXT NOT NULL,           -- 'ok', 'degraded', 'disconnected'
    last_error TEXT,
    metadata TEXT                   -- JSON
);

CREATE UNIQUE INDEX idx_sources_unique ON sources(kind, distro, tmux_socket);
CREATE INDEX idx_sources_status ON sources(status);

-- Core session tracking (separate stable identity from display name)
CREATE TABLE sessions (
    session_uid TEXT PRIMARY KEY,  -- stable internal id (uuidv7)
    source_id TEXT NOT NULL,
    tmux_session_id TEXT,          -- tmux "#{session_id}" (e.g. "$1") when available
    name TEXT NOT NULL,            -- ntm session name (display)
    created_at INTEGER NOT NULL,   -- unix timestamp
    last_seen_at INTEGER NOT NULL,
    ended_at INTEGER,              -- null if active
    status TEXT NOT NULL,          -- 'active', 'idle', 'ended', 'unknown'
    status_reason TEXT,            -- short machine-readable reason
    pane_count INTEGER DEFAULT 0,
    metadata TEXT,                 -- JSON: agent types, working dir, etc.
    FOREIGN KEY (source_id) REFERENCES sources(source_id) ON DELETE CASCADE
);

-- Enforce at most one active session per name per source while still allowing historical reuse
CREATE UNIQUE INDEX idx_sessions_name_active ON sessions(source_id, name) WHERE ended_at IS NULL;
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_source ON sessions(source_id);

-- Individual pane tracking (avoid collisions from pane index reuse)
CREATE TABLE panes (
    pane_uid TEXT PRIMARY KEY,     -- stable internal id (uuidv7)
    session_uid TEXT NOT NULL,
    tmux_pane_id TEXT,             -- tmux "#{pane_id}" (e.g. "%12") when available
    tmux_window_id TEXT,           -- tmux "#{window_id}" (e.g. "@3") when available
    tmux_pane_pid INTEGER,         -- tmux "#{pane_pid}" when available
    pane_index INTEGER NOT NULL,   -- display-only; unstable across splits/reorders
    agent_type TEXT,               -- 'claude', 'codex', 'gemini', 'shell'
    created_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    last_activity_at INTEGER,      -- derived from #{pane_last_activity} (ms)
    current_command TEXT,          -- #{pane_current_command}
    ended_at INTEGER,
    status TEXT NOT NULL,          -- 'active', 'waiting', 'idle', 'ended', 'unknown'
    status_reason TEXT,
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid) ON DELETE CASCADE
);

CREATE INDEX idx_panes_session ON panes(session_uid);
CREATE INDEX idx_panes_status ON panes(status);
CREATE UNIQUE INDEX idx_panes_session_tmux ON panes(session_uid, tmux_pane_id)
  WHERE tmux_pane_id IS NOT NULL;

-- Minute-resolution samples (prevents explosive growth at scale)
-- Keep detailed samples for a short window; roll up aggressively.
CREATE TABLE pane_minute_samples (
    minute_start INTEGER NOT NULL, -- unix timestamp (UTC), start of minute
    pane_uid TEXT NOT NULL,
    status TEXT NOT NULL,
    output_lines INTEGER DEFAULT 0,
    output_bytes INTEGER DEFAULT 0,
    estimated_tokens INTEGER DEFAULT 0,
    PRIMARY KEY (minute_start, pane_uid),
    FOREIGN KEY (pane_uid) REFERENCES panes(pane_uid) ON DELETE CASCADE
);

CREATE INDEX idx_pane_minute_time ON pane_minute_samples(minute_start);

-- Hourly aggregates for usage graphs
CREATE TABLE hourly_stats (
    hour_start INTEGER NOT NULL,   -- unix timestamp, start of hour
    session_uid TEXT NOT NULL,
    total_compacts INTEGER DEFAULT 0,
    active_minutes INTEGER DEFAULT 0,
    estimated_tokens INTEGER DEFAULT 0,
    PRIMARY KEY (hour_start, session_uid),
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid)
);

-- Daily aggregates for usage graphs
CREATE TABLE daily_stats (
    day_start INTEGER NOT NULL,    -- unix timestamp (UTC), start of day
    tz_offset_min INTEGER NOT NULL,-- offset used when bucketing (DST-safe reporting)
    session_uid TEXT NOT NULL,
    total_compacts INTEGER DEFAULT 0,
    active_minutes INTEGER DEFAULT 0,
    estimated_tokens INTEGER DEFAULT 0,
    PRIMARY KEY (day_start, session_uid),
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid)
);

-- Ordered event log (compacts, escalations, pane status changes)
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT, -- monotonic cursor
    session_uid TEXT NOT NULL,
    pane_uid TEXT NOT NULL,
    type TEXT NOT NULL,                   -- 'compact', 'escalation', 'pane.status', ...
    detected_at INTEGER NOT NULL,
    source TEXT NOT NULL,                 -- 'ntm', 'tmux', 'stream', 'heuristic'
    confidence REAL DEFAULT 1.0,          -- 0.0 - 1.0
    severity TEXT,                        -- 'info', 'warn', 'error'
    status TEXT,                          -- for escalations: 'pending', 'resolved', 'dismissed'
    resolved_at INTEGER,
    trigger TEXT,                         -- for compacts: 'auto', 'manual', 'error'
    message TEXT,
    context_before INTEGER,
    payload TEXT,                         -- JSON for extensibility
    dedupe_hash TEXT,                     -- prevents duplicate inserts on reconnect/restart
    FOREIGN KEY (pane_uid) REFERENCES panes(pane_uid) ON DELETE CASCADE,
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_hourly_time ON hourly_stats(hour_start);
CREATE INDEX idx_daily_time ON daily_stats(day_start);
CREATE INDEX idx_events_pane_time ON events(pane_uid, detected_at);
CREATE INDEX idx_events_type_time ON events(type, detected_at);
CREATE INDEX idx_events_source_time ON events(source, detected_at);
CREATE INDEX idx_events_status ON events(status);
CREATE UNIQUE INDEX idx_events_dedupe ON events(dedupe_hash) WHERE dedupe_hash IS NOT NULL;
```

### 2.2 TypeScript Types

```typescript
// Core types
interface Source {
  id: string;        // source_id
  kind: 'wsl';       // future: 'ssh'
  distro: string;    // e.g. 'Ubuntu'
  tmuxSocket?: string;
  lastSeenAt: Date;
  status: 'ok' | 'degraded' | 'disconnected';
  lastError?: string;
}

interface Session {
  id: string;        // session_uid
  sourceId: string;  // source_id
  tmuxSessionId?: string;
  name: string;      // ntm session name (display)
  createdAt: Date;
  lastSeenAt: Date;
  endedAt?: Date;
  status: 'active' | 'idle' | 'ended' | 'unknown';
  statusReason?: string;
  paneCount: number;
  panes: Pane[];
  metadata: SessionMetadata;
}

interface SessionMetadata {
  workingDir?: string;
  agentConfig?: string;  // e.g., "--cc=2 --cx=1"
  tags?: string[];
}

interface Pane {
  id: string;        // pane_uid
  sessionId: string; // session_uid
  tmuxPaneId?: string;
  tmuxWindowId?: string;
  tmuxPid?: number;
  paneIndex: number;
  agentType: 'claude' | 'codex' | 'gemini' | 'shell' | 'unknown';
  createdAt: Date;
  lastSeenAt: Date;
  endedAt?: Date;
  status: 'active' | 'waiting' | 'idle' | 'ended' | 'unknown';
  statusReason?: string;
  currentCommand?: string;
  stats: PaneStats;
}

interface PaneStats {
  compactCount: number;
  totalTokensEstimate: number;
  activeMinutes: number;
  lastActivity: Date;
  currentContextSize: number;  // estimated
}

interface Event {
  id: number;
  sessionId: string;
  paneId: string;
  detectedAt: Date;
  source: 'ntm' | 'tmux' | 'stream' | 'heuristic';
  confidence: number; // 0.0 - 1.0
  severity?: 'info' | 'warn' | 'error';
  resolvedAt?: Date;
  type: 'compact' | 'escalation' | 'pane.status';
  display?: { sessionName?: string; paneIndex?: number }; // optional UI convenience
  status?: 'pending' | 'resolved' | 'dismissed';
  trigger?: 'auto' | 'manual' | 'error';
  message?: string;
  contextBefore?: number;
  payload?: Record<string, unknown>;
}

// API response types
interface DashboardData {
  sources: Source[];
  sessions: Session[];
  totalActivePanes: number;
  totalCompactsToday: number;
  activeHoursToday: number;
  pendingEscalations: number;
}

interface HourlyStats {
  hour: Date;
  compacts: number;
  activeMinutes: number;
  tokens: number;
}

interface DailyStats {
  day: Date;
  compacts: number;
  activeMinutes: number;
  tokens: number;
}
```

---

## 3. API Design

### 3.1 Canonical RPC (JSON-RPC 2.0)

```yaml
Transports:
  - stdio (default): length-prefixed framed JSON messages (full duplex; supports notifications)
  - WebSocket (optional): ws://localhost:3847/ws?token=...
  - HTTP (optional): POST http://localhost:3847/rpc  (JSON-RPC body)

Auth:
  - WS/HTTP require bearer token
  - stdio may run tokenless (explicit config) but should still support token for parity

Methods (canonical; stable IDs):
  - health.get
  - capabilities.get
  - snapshot.get         # cold-start: dashboard state + lastEventId
  - sessions.list
  - sessions.get
  - panes.get            # by paneId (pane_uid), not pane index
  - panes.outputPreview  # by paneId
  - events.list          # cursor-based, resumable
  - subscribe            # realtime subscription management (stdio or WS)
  - escalations.list
  - escalations.dismiss
  - stats.summary
  - stats.hourly
  - stats.daily
  - config.get           # admin or read-only depending on policy
  - config.set           # admin
  - config.reload        # admin
  - detectors.list       # available detector packs + versions
  - detectors.reload     # admin
  - actions.sessionKill  # admin, optional
  - actions.paneSend     # admin, optional (paneId)
  - attach.command       # paneId -> tmux attach + select-pane
```

Notes:
- `sessionId` refers to `session_uid`; `paneId` refers to `pane_uid`
- `paneIndex` is display-only metadata and should never be used for actions

**HTTP convenience endpoints (when HTTP transport enabled):**

```yaml
GET /health
  Response: {
    status: "ok" | "degraded",
    uptime: 3600,
    version: "1.0.0",
    instanceId: "…",
    runId: "…",
    schemaVersion: 3,
    protocolVersion: 1,
    capabilities: { ntm: true, tmux: true, stream: false, systemd: true },
    lastTmuxPollOkAt: 1737612345,
    lastNtmPollOkAt: 1737612345,
    lastError?: string
  }

GET /debug/diagnostics   # admin only
  Response: { version: string, instanceId: string, lastPollOkAt: number, lastError?: string, queueDepth: number }

GET /debug/self-test     # admin only
  Response: { ok: boolean, checks: { name: string, ok: boolean, detail?: string }[] }

GET /debug/metrics       # admin only
  Response: { counters: Record<string, number>, gauges: Record<string, number>, timingsMs: Record<string, { p50: number, p95: number, max: number }> }

GET /debug/log-tail      # admin only
  Query: ?lines=200
  Response: { lines: string[] }
```

### 3.2 WebSocket Notifications (JSON-RPC)

```typescript
// Connection (WS) : ws://localhost:3847/ws?token=...
// Connection (stdio): same framing as requests; notifications arrive on the same stream after `subscribe`

// Client -> Server: subscribe request (resume-safe)
{ "jsonrpc": "2.0", "id": 1, "method": "subscribe", "params": { "channels": ["sessions", "events"], "sinceEventId": 123 } }

// Server -> Client: hello notification (capabilities + last cursor)
{ "jsonrpc": "2.0", "method": "hello", "params": { "instanceId": "…", "runId": "…", "version": "1.0.0", "protocolVersion": 1, "lastEventId": 12345 } }

// Server -> Client: heartbeat (every N seconds)
{ "jsonrpc": "2.0", "method": "ping", "params": { "t": 1737612345 } }

// Client -> Server: heartbeat response
{ "jsonrpc": "2.0", "method": "pong", "params": { "t": 1737612345 } }

// Server -> Client: session delta update
{
  "jsonrpc": "2.0",
  "method": "session.delta",
  "params": {
    "sessionId": "sess_01HF...",
    "status": "active",
    "panes": [...]
  }
}

// Server -> Client: event batch (bounded)
{
  "jsonrpc": "2.0",
  "method": "events",
  "params": {
    "events": [
      {
        "id": 12346,
        "type": "compact",
        "sessionId": "sess_01HF...",
        "paneId": "pane_01HF...",
        "detectedAt": "2026-01-23T06:30:00Z",
        "trigger": "auto",
        "contextBefore": 95000,
        "display": { "sessionName": "my-session", "paneIndex": 0 }
      }
    ],
    "nextEventId": 12347
  }
}
```

### 3.3 Versioning, Errors, and Schema Generation

The plan currently specifies a protocol and a handful of methods, but it does not define compatibility or error semantics. Without that, upgrades and reconnects will be fragile.

Versioning:
- protocolVersion: increment on breaking JSON-RPC changes (method removal, param shape changes).
- schemaVersion: tracks SQLite migrations and storage changes.
- capabilities: feature flags plus relevant versions (tmux, ntm, detector packs, capture backends).

Cold start and resume:
- snapshot.get returns the full dashboard state plus lastEventId so the UI can render immediately.
- subscribe supports sinceEventId and must be resume-safe; server replies with hello { lastEventId }.
- events.list is the fallback for HTTP and for resync after missed notifications.

Error model:
Keep errors machine-readable (code + detail); do not force the UI to parse strings. Suggested stable error codes:
- UNAUTHORIZED (token missing or invalid)
- FORBIDDEN (admin methods without admin token)
- RATE_LIMITED (server-side throttle)
- STALE_CURSOR (client requested events older than retention)
- UNSUPPORTED (method not supported on this daemon build)
- DEGRADED (request served, but collectors are unhealthy)

Schema as a contract:
- Maintain a JSON Schema for RPC requests and responses in shared/.
- Generate Rust and TypeScript bindings from that schema and validate in debug builds.

---

## 4. Detection Algorithms

### 4.1 Compact Detection

```typescript
// Patterns indicating a context compact/reset occurred.
// Split into "hard" signals (emit compact) and "warning" signals (optional future event).
const COMPACT_HARD_PATTERNS = [
  // Claude Code specific
  /Auto-compacting conversation/i,
  /Conversation compacted/i,
  /Context limit reached/i,

  // Generic patterns
  /\bcompacting\b/i,
  /\bconversation\b.*\bcompacted\b/i,
  /\bcontext\b.*\breset\b/i,
  /\bstarting\s+fresh\s+context\b/i,
  /\bcontext\s+window\b.*\b(full|exceeded)\b/i,
];

const CONTEXT_WARNING_PATTERNS = [
  /approaching context limit/i,
  /\bcontext\b.*\b(nearing|close to)\b.*\blimit\b/i,
  /\b(\d{2,3})k\s*\/\s*(\d{2,3})k\s*tokens\b/i,  // warning only; too ambiguous to treat as a compact by itself
];

// Detection logic
function detectCompact(
  newChunk: string,              // incremental bytes/lines (stream mode) or sampled tail when enabled
  paneState: PaneState,
  detectorState: DetectorState   // includes debounce + rolling hashes
): Event | null {
  // Method 1: Pattern matching on incremental output (ANSI stripped)
  const clean = stripAnsi(newChunk);
  for (const pattern of COMPACT_HARD_PATTERNS) {
    if (pattern.test(clean) && detectorState.allow("compact", 60_000)) {
      return {
        type: 'compact',
        sessionId: paneState.sessionId,
        paneId: paneState.id,
        detectedAt: new Date(),
        trigger: 'auto',
        contextBefore: paneState.estimatedTokens
      };
    }
  }

  // Method 2: Prefer structured sources when available.
  // If NTM provides a per-pane compact counter or context size, treat changes in those fields as the source of truth.
  const m = paneState.ntmMetrics; // optional: { compactCount?: number; contextTokens?: number; contextLimit?: number }
  const prev = detectorState.prevNtmMetrics(paneState.id);

  if (m && prev) {
    if (m.compactCount != null && prev.compactCount != null && m.compactCount > prev.compactCount) {
      detectorState.setPrevNtmMetrics(paneState.id, m);
      if (detectorState.allow("compact", 10_000)) {
        return {
          type: 'compact',
          sessionId: paneState.sessionId,
          paneId: paneState.id,
          detectedAt: new Date(),
          trigger: 'auto',
          contextBefore: prev.contextTokens ?? paneState.estimatedTokens,
          payload: { detector: 'ntm.counter', confidence: 0.95 }
        };
      }
    }

    // Heuristic fallback: big context drop suggests a reset even if the counter is missing.
    if (m.contextTokens != null && prev.contextTokens != null) {
      const drop = prev.contextTokens - m.contextTokens;
      const ratio = m.contextTokens / Math.max(1, prev.contextTokens);
      if (prev.contextTokens > 20_000 && drop > 10_000 && ratio < 0.25 && detectorState.allow("compact", 10_000)) {
        detectorState.setPrevNtmMetrics(paneState.id, m);
        return {
          type: 'compact',
          sessionId: paneState.sessionId,
          paneId: paneState.id,
          detectedAt: new Date(),
          trigger: 'auto',
          contextBefore: prev.contextTokens,
          payload: { detector: 'ntm.context-drop', confidence: 0.75, prev: prev.contextTokens, curr: m.contextTokens }
        };
      }
    }

    detectorState.setPrevNtmMetrics(paneState.id, m);
  }

  return null;
}
```

### 4.2 Agent Type Detection

```typescript
function detectAgentType(paneOutput: string): AgentType {
  // Check for distinctive prompts/patterns
  if (/claude>|Claude Code/i.test(paneOutput)) return 'claude';
  if (/codex>|OpenAI Codex/i.test(paneOutput)) return 'codex';
  if (/gemini>|Google Gemini/i.test(paneOutput)) return 'gemini';
  if (/\$\s*$|bash-\d|#\s*$/m.test(paneOutput)) return 'shell';
  return 'unknown';
}
```

### 4.3 Pane Status Detection (tmux-metadata first)

```typescript
type TmuxPaneMeta = {
  lastActivityEpochMs: number;   // from #{pane_last_activity} * 1000
  currentCommand?: string;       // from #{pane_current_command}
  inMode?: boolean;              // from #{pane_in_mode}
  dead?: boolean;                // from #{pane_dead}
};

function detectPaneStatus(outputTail: string, meta: TmuxPaneMeta): PaneStatus {
  if (meta.dead) return 'ended';
  const idleThresholdMs = 5 * 60 * 1000;
  const timeSinceActivity = Date.now() - meta.lastActivityEpochMs;

  // Check for waiting indicators (only when activity is recent)
  const waitingPatterns = [
    /waiting for input/i,
    /\(y\/n\)/i,
    /press enter/i,
    />\s*$/,  // Prompt waiting
  ];

  if (waitingPatterns.some(p => p.test(outputTail.slice(-500))) && timeSinceActivity < idleThresholdMs) {
    return 'waiting';
  }

  // Check for active processing
  const activePatterns = [
    /thinking\.\.\./i,
    /processing/i,
    /reading.*file/i,
    /executing/i,
  ];

  if (activePatterns.some(p => p.test(outputTail.slice(-200)))) {
    return 'active';
  }

  // Fall back to time-based
  return timeSinceActivity > idleThresholdMs ? 'idle' : 'active';
}
```

### 4.4 Tmux Metadata Ingestion

Use `tmux list-panes -a -F` to fetch stable pane signals without parsing output:
- pane id: `#{pane_id}`
- last activity: `#{pane_last_activity}`
- current command: `#{pane_current_command}`
- dead/in-mode flags: `#{pane_dead}` / `#{pane_in_mode}`

This reduces CPU, improves correctness, and makes status changes resilient to agent output format changes.

### 4.5 Escalation Detection

```typescript
const ESCALATION_PATTERNS = [
  // Direct requests for human input
  /need.*human.*input/i,
  /escalating to user/i,
  /requires manual intervention/i,

  // Confirmation prompts (avoid overly broad matches)
  /\bplease\s+confirm\b.*\b(delete|remove|overwrite|proceed|continue)\b/i,
  /\bconfirm\b.*\b(delete|remove|overwrite|proceed|continue)\b/i,

  // Error states
  /fatal error/i,
  /cannot proceed/i,
  /permission denied.*continue\?/i,
];

function detectEscalation(newChunk: string, meta: TmuxPaneMeta, detectorState: DetectorState): string | null {
  const recentOutput = stripAnsi(newChunk).slice(-2000);

  // Avoid firing on conversational confirmations unless the pane is plausibly waiting for input.
  const promptTail = recentOutput.slice(-500);
  const looksLikePrompt =
    /y\/n/i.test(promptTail) ||
    /press enter/i.test(promptTail) ||
    />\s*$/.test(promptTail);
  const activityIsRecent = (Date.now() - meta.lastActivityEpochMs) < 5 * 60 * 1000;
  for (const pattern of ESCALATION_PATTERNS) {
    const match = recentOutput.match(pattern);
    if (match) {
      if (!detectorState.allow("escalation", 30_000)) return null;

      if (!(looksLikePrompt && activityIsRecent) && !/fatal error|cannot proceed/i.test(match[0])) return null;
      // Extract surrounding context for message
      const idx = recentOutput.lastIndexOf(match[0]);
      const start = Math.max(0, idx - 100);
      const end = Math.min(recentOutput.length, idx + match[0].length + 100);
      return recentOutput.slice(start, end).trim();
    }
  }
  return null;
}
```

### 4.6 Optional tmux Hook Integration (event-driven fast path)

Polling is the portability baseline, but tmux can emit lifecycle and focus events directly via hooks. Using hooks as a hint stream improves responsiveness and lets you lower polling frequency on large servers.

**Design constraints:**
- Hooks must be **optional** and must not break existing user hooks or tmux workflows.
- Hooks are **hints**, not truth; the daemon still reconciles against periodic `list-panes` snapshots to prevent drift.
- Hook commands must be **non-blocking**; use background execution and append-only writes.

**Suggested hooks to consume:**
- `session-created`, `session-closed`
- `pane-died`, `pane-exited`
- `pane-focus-in`, `pane-focus-out`
- `window-pane-changed` (split/resize is still best detected via polling)

**Emission format (example):**
- Append one JSON line per hook event to a file in `~/.local/share/ntm-tracker/hooks.log`.
- Daemon tails this file and uses it to trigger immediate lightweight refresh (not a full reconcile).

If hooks are unavailable or conflict with user configuration, disable them and rely on polling only.

---

## 5. UI Design

### 5.1 System Tray

```
┌─────────────────────────────────────┐
│ [NTM Icon with badge: 3]            │  <- 3 = active sessions
│                                     │
│ Tooltip on hover:                   │
│ "3 sessions, 7 panes active         │
│  2 compacts today | 4.2h usage     │
│  daemon: ok (Ubuntu) | last: 2s"    │
└─────────────────────────────────────┘

Right-click menu:
┌─────────────────────────────────┐
│ ● 3 Active Sessions             │
│   Connection: OK (Ubuntu)       │
│ ─────────────────────────────── │
│   my-project (2 panes)      ▶   │
│   research (3 panes)        ▶   │
│   experiments (2 panes)     ▶   │
│ ─────────────────────────────── │
│   Open Dashboard                │
│   Search Sessions…              │
│   Snooze Notifications ▶        │
│   Pin Sessions ▶                │
│   ─────────────────────────     │
│   Open Logs                     │
│   Diagnostics (Export Bundle)   │
│   Restart Daemon                │
│   Reconnect                      │
│   Settings                      │
│   Quit                          │
└─────────────────────────────────┘

Session submenu:
┌─────────────────────────────────┐
│ my-project                      │
│ ─────────────────────────────── │
│ Pane 0: claude (active)     ◉   │
│ Pane 1: claude (idle)       ○   │
│ ─────────────────────────────── │
│ Compacts: 3 | Time: 2.5h        │
│ ─────────────────────────────── │
│ Focus Active Pane               │
│ Copy Attach Cmd                 │
│ Open in Terminal                │
└─────────────────────────────────┘
```

### 5.2 Dashboard Window

```
┌──────────────────────────────────────────────────────────────────────────┐
│ NTM Tracker                                                    [_][□][X] │
├──────────────────────────────────────────────────────────────────────────┤
│  Search: [____________________]  Source: [Ubuntu ▾]  Status: [All ▾]     │
│                                                                          │
│  ┌─ Overview ──────────────────────────────────────────────────────────┐ │
│  │                                                                     │ │
│  │   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐           │ │
│  │   │    3    │   │    7    │   │    5    │   │  4.2h   │           │ │
│  │   │Sessions │   │  Panes  │   │Compacts │   │  Today  │           │ │
│  │   └─────────┘   └─────────┘   └─────────┘   └─────────┘           │ │
│  │                                                                     │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─ Sessions ───────────────────────────────────────────────────────────┐│
│  │                                                                      ││
│  │  ┌─ my-project ─────────────────────────────────────────────────┐   ││
│  │  │ Status: Active │ Panes: 2 │ Compacts: 3 │ Runtime: 2h 34m    │   ││
│  │  │                                                              │   ││
│  │  │  ┌────────────────────────────────────────────────────────┐  │   ││
│  │  │  │ Pane 0 │ claude │ ● Active │ Compacts: 2 │ ~45k tokens │  │   ││
│  │  │  │ Pane 1 │ claude │ ○ Idle   │ Compacts: 1 │ ~12k tokens │  │   ││
│  │  │  └────────────────────────────────────────────────────────┘  │   ││
│  │  │                                                              │   ││
│  │  │  [Open in Terminal] [View Output] [Kill Session]             │   ││
│  │  └──────────────────────────────────────────────────────────────┘   ││
│  │                                                                      ││
│  │  ┌─ research ───────────────────────────────────────────────────┐   ││
│  │  │ Status: Active │ Panes: 3 │ Compacts: 1 │ Runtime: 45m       │   ││
│  │  │ ...                                                          │   ││
│  │  └──────────────────────────────────────────────────────────────┘   ││
│  │                                                                      ││
│  └──────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  ┌─ Activity (24h) ─────────────────────────────────────────────────────┐│
│  │     ▁▂▃▅▇█▇▅▃▂▁▁▁▁▂▃▄▅▆▇█▇▅▃                                       ││
│  │     ─────────────────────────────────────────────────────────────   ││
│  │     06  08  10  12  14  16  18  20  22  00  02  04  06             ││
│  │                                                                      ││
│  │     ■ Compacts    ■ Active time                                     ││
│  └──────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  ┌─ Timeline (selected session/pane) ───────────────────────────────────┐│
│  │  06:28  pane.status  active                                          ││
│  │  06:30  compact       auto (context ~95k)                            ││
│  │  06:33  escalation    pending: "Please confirm..."                   ││
│  └──────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  ┌─ Escalation Inbox (1 pending) ────────────────────────────────────────┐│
│  │  ⚠ research:1  "Please confirm deletion..."                          ││
│  │   [Focus] [Copy Attach Cmd] [Snooze 15m] [Dismiss]            2m ago ││
│  └──────────────────────────────────────────────────────────────────────┘│
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Notification Toasts

```
┌────────────────────────────────────┐
│ 🔄 Context Compacted               │
│ my-project:0 auto-compacted        │
│ (was ~95k tokens)                  │
│                          just now  │
└────────────────────────────────────┘

┌────────────────────────────────────┐
│ ⚠️ Escalation                       │
│ research:1 needs attention         │
│ "Please confirm you want to..."    │
│                          [Focus]   │
└────────────────────────────────────┘
```

### 5.4 Settings & Rules Editor (compelling usefulness beyond a viewer)

A tracker that cannot be tuned becomes noise. The UI should expose the notification and capture policies as first-class, validated settings instead of raw JSON.

Must-have UX:
- Rule editor with preview: "show me which sessions this rule matches".
- Per-session overrides without requiring stable internal IDs in the UI (use name mapping but store UID).
- Clear indicators when capture is enabled; require explicit opt-in per session.
- A visible health banner when the daemon is degraded or disconnected.

This reduces false positives, reduces alert fatigue, and makes the tool usable long-term.

---

## 6. Implementation Phases

### Phase -1: Risk Spikes (before feature work)
**Goal**: Validate the two biggest unknowns early; lock protocol and polling strategy

**Tasks**:
- [ ] Confirm NTM robot outputs available in target versions (tokens, compacts, pane identifiers)
- [ ] Benchmark tmux metadata polling cost on 20 sessions x 6 panes
- [ ] Measure tmux/ntm command latency distribution (p50/p95/p99) and set explicit timeouts
- [ ] Validate WSL localhost connectivity on Windows 10 and 11; confirm fallback path
- [ ] Produce fixture corpus from real sessions for parser and detector tests
- [ ] Add parser fuzz harness (cargo-fuzz) seeded with fixtures; run in CI to catch format drift

**Exit criteria**:
- [ ] One chosen default transport + one tested fallback
- [ ] Measured CPU baseline and command latency distribution
- [ ] Realtime push validated over stdio without relying on WSL localhost forwarding

### Phase 0: Install & Versioning (before feature work)
**Goal**: Deterministic daemon install, protocol negotiation, and safe upgrades

**Tasks**:
- [ ] First-run bootstrap: Windows app installs daemon into WSL (no external runtime assumptions)
- [ ] Version handshake: UI refuses to operate on incompatible protocol/schema versions
- [ ] Upgrade path: daemon can be updated by the Windows app (atomic replace + restart)
- [ ] Rollback: keep previous daemon binary/version for quick recovery

### Phase 1: Foundation (Days 1-2)
**Goal**: Working daemon with data collection

#### Day 1: Project Setup & Core Daemon

```bash
# Directory structure
ntm-tracker/
├── daemon/                 # WSL2 service (Rust release binary)
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── rpc.rs         # JSON-RPC handlers + transport adapters
│   │   ├── collector.rs   # tmux + ntm collection loop(s)
│   │   ├── detector.rs    # Detection engine
│   │   ├── db.rs          # SQLite wrapper + migrations
│   │   └── types.rs       # Generated protocol types (or shared crate)
│   └── Cargo.toml
├── app/                    # Tauri app (Phase 2)
├── shared/                 # Protocol schema (JSON Schema) + generated bindings
└── PLAN.md
```

**Tasks**:
- [ ] Initialize Rust workspace for daemon (tokio + sqlite)
- [ ] Build and publish a pinned Linux release artifact (single daemon binary)
- [ ] Implement SQLite schema and migrations
- [ ] Create NTM command wrapper (`ntm --robot-*` parsing)
- [ ] Build tmux fast metadata loop + adaptive NTM reconcile
- [ ] Implement session/pane state tracking
- [ ] Add structured logging + timing metrics (configurable verbosity)
- [ ] Add minimal CLI commands for health/status (use same RPC as UI)
- [ ] Add detector pack loader (local file first; UI-managed updates later)

#### Day 2: Detection & API

**Tasks**:
- [ ] Implement compact detection algorithm
- [ ] Implement pane status detection
- [ ] Implement escalation detection
- [ ] Implement canonical JSON-RPC handlers (health/sessions/events/stats/actions)
- [ ] Add transport adapters: stdio (default), WebSocket (optional), HTTP POST /rpc (optional)
- [ ] Add diagnostics, self-test, and metrics endpoints (admin)
- [ ] Add service management options:
      - systemd unit (when available)
      - Windows-supervised launch via `wsl.exe` (preferred default)
      - single-instance guard (pidfile/lock) + graceful shutdown handling
- [ ] Write integration tests with mock ntm output
- [ ] Add golden tests for parser outputs and detector triggers (fixture corpus committed)
- [ ] Add fuzz tests for NTM parser and detector pack loader (reject malformed inputs safely)

**Deliverable**: Running daemon with stdio JSON-RPC by default, plus optional WS/HTTP at `localhost:3847`.

---

### Phase 2: Windows App (Days 3-5)
**Goal**: Working Tauri app with system tray

#### Day 3: Tauri Setup & System Tray

**Tasks**:
- [ ] Initialize Tauri 2.0 project
- [ ] Configure system tray with icon
- [ ] Implement tray menu structure
- [ ] Add transport client (stdio default; TCP/WS optional)
- [ ] Implement daemon bootstrap logic (start/stop/healthcheck via `wsl.exe` and/or TCP)
- [ ] Display basic session info in tooltip
- [ ] Handle Windows autostart

#### Day 4: Dashboard UI

**Tasks**:
- [ ] Set up Svelte 5 with Tailwind
- [ ] Build overview cards component
- [ ] Build session list component
- [ ] Build pane details component
- [ ] Implement real-time updates via `subscribe` over the active transport (stdio default; WS optional)
- [ ] Add expand/collapse for sessions

#### Day 5: Polish & Notifications

**Tasks**:
- [ ] Implement Windows toast notifications
- [ ] Add activity graph (Chart.js or similar)
- [ ] Build escalation panel
- [ ] Add settings panel (poll interval, notifications)
- [ ] Add diagnostics export bundle + restart daemon action
- [ ] Package and test installer
- [ ] Handle daemon connection errors gracefully

**Deliverable**: Installable Windows app with full functionality

---

### Phase 3: Refinement (Days 6-7)
**Goal**: Production-ready release

#### Day 6: Testing & Edge Cases

**Tasks**:
- [ ] Test with multiple concurrent sessions
- [ ] Test daemon restart recovery
- [ ] Chaos test: kill -9 during write; validate sqlite WAL recovery and no corrupted state
- [ ] Test Windows app crash recovery
- [ ] Handle WSL2 network quirks
- [ ] Add retry logic for API calls
- [ ] Performance profiling
- [ ] Load test: 20 sessions x 6 panes; measure CPU, WS event latency, DB growth
- [ ] Integration test with real tmux session in CI (Linux runner)
- [ ] Enforce performance budgets as release gates (instrumented from Phase 1):
      - idle daemon CPU: <= 1% on typical laptop
      - end-to-end update latency: p95 <= 500ms under load target
      - DB growth: bounded by config; events and samples pruned by retention policy
- [ ] Test matrix:
      - Windows 10/11; WSL2 with and without systemd
      - Ubuntu and at least one non-Ubuntu distro
      - NTM installed vs missing (tmux-only mode)

#### Day 7: Documentation & Release

**Tasks**:
- [ ] Write README with install instructions
- [ ] Create release build pipeline
- [ ] CI: Windows build + smoke test for both modes:
      - stdio mode: start daemon via `wsl.exe`, run `health.get`, verify `subscribe` notifications
      - service mode (optional): WS subscribe works when enabled
- [ ] Sign Windows installer and verify update integrity (Tauri updater)
- [ ] Verify daemon artifact integrity before installing into WSL (hash + signature)
- [ ] Add configuration documentation
- [ ] Create demo GIF/video
- [ ] Final testing on clean Windows install

---

## 7. Configuration

### 7.1 Daemon Config (`~/.config/ntm-tracker/daemon.toml`)

```toml
[server]
port = 3847
host = "127.0.0.1"  # Default: do not bind publicly; prefer stdio transport

[security]
require_auth = true
read_token_file = "~/.config/ntm-tracker/token.read"
admin_token_file = "~/.config/ntm-tracker/token.admin"
max_request_body_kb = 64
rate_limit_rps = 25
enable_actions = false          # default off; UI can prompt to enable
token_rotate_on_start = true
enforce_token_file_permissions = true  # refuse to start if token files are not 0600

[exec]
tmux_timeout_ms = 2000          # fast commands only; treat timeouts as degraded health
ntm_timeout_ms = 10000          # ntm status + robot-tail; slower and more variable
ntm_markdown_timeout_ms = 20000 # robot-markdown is diagnostic and slower
max_stdout_kb = 256             # prevent runaway capture
kill_on_timeout = true

[polling]
tmux_meta_interval_ms = 1500    # Cheap: pane_last_activity, pane_dead, current_command
ntm_reconcile_interval_ms = 15000 # Rich but heavier; adaptive backoff per session
idle_backoff_max_ms = 60000     # Upper bound when idle
max_concurrent_commands = 4     # Prevent spawn storms
sample_interval_ms = 60000      # Persist minute-level samples (bounded growth)
capture_fallback_interval_ms = 30000 # Fallback capture when stream mode is unavailable
history_retention_days = 30     # How long to keep hourly/daily aggregates

[hooks]
enable = false                 # Optional; use as hint stream, not source of truth
event_log = "~/.local/share/ntm-tracker/hooks.log"
max_event_log_mb = 5
install_mode = "append"        # append | replace | off (avoid clobbering user hooks)

[maintenance]
rollup_interval_ms = 3600000     # hourly rollups + daily boundary checks
vacuum_interval_hours = 168      # weekly vacuum (or incremental vacuum)
max_db_mb = 512                 # hard cap; prune oldest minute samples first
minute_samples_retention_hours = 72
events_retention_days = 30
sessions_retention_days = 90

[detection]
compact_patterns = [
  "Auto-compacting conversation",
  "Context limit reached",
]
escalation_patterns = [
  "need.*human.*input",
  "please confirm",
]

[logging]
level = "info"  # debug, info, warn, error
file = "~/.local/share/ntm-tracker/daemon.log"
max_file_mb = 10
max_files = 5
format = "json"           # json | text

[capture]
mode = "off"                     # Default: metadata-only. Enable per-session when needed.
persist_preview = false          # Default: never persist pane output; generate preview on demand and redact
stream_backend = "fifo"          # "fifo" | "disk"
max_preview_lines = 200          # UI preview cap
max_preview_kb = 64              # UI preview cap

[privacy]
per_session_capture_allowlist = []   # session names or session_uids
show_capture_enabled_banner = true

[redaction]
patterns = ["AKIA[0-9A-Z]{16}", "-----BEGIN PRIVATE KEY-----"]
replacement = "[REDACTED]"
max_scan_bytes = 262144           # bound CPU cost; redact only the most recent bytes in previews/log tails
apply_to = ["output_preview", "diagnostics_bundle", "logs"]

[stream_limits]
max_total_mb = 256               # total disk usage for stream mode
rotate_mb = 5
max_files_per_pane = 5
```

### 7.2 App Config (stored in Windows AppData)

```json
{
  "daemon": {
    "url": "http://localhost:3847",
    "readTokenRef": "<stored in Windows Credential Manager>",
    "adminTokenRef": "<stored in Windows Credential Manager>",
    "transport": "wsl-stdio",
    "fallbackTransport": "tcp",
    "reconnectIntervalMs": 5000
  },
  "wsl": {
    "distro": "Ubuntu",
    "startDaemonOnLaunch": true
  },
  "ui": {
    "startMinimized": true,
    "showNotifications": true,
    "notifyOnCompact": true,
    "notifyOnEscalation": true,
    "theme": "system"
  },
  "notifications": {
    "quietHours": { "enabled": true, "start": "22:00", "end": "07:00" },
    "dedupeWindowMs": 300000,
    "snoozeMinutesDefault": 15,
    "rules": [
      { "type": "escalation", "minPendingMs": 120000, "includeSessions": ["*"], "excludeSessions": [] },
      { "type": "compact", "maxPerHour": 3, "includeSessions": ["*"], "excludeSessions": ["research"] }
    ]
  },
  "perSession": {
    "overrides": {
      "sess_01HF...": { "notifyOnEscalation": true, "capturePreview": false },
      "sess_01HG...": { "notifyOnCompact": false }
    }
  },
  "tray": {
    "showBadge": true,
    "badgeContent": "sessions",
    "maxSessionsShown": 10
  }
}
```

### 7.3 Privacy Model & Diagnostics Bundle Contract

The plan already defaults capture to off, but it does not define what is stored, where redaction happens, and what the diagnostics bundle guarantees. Make this explicit to avoid accidental leakage.

Data minimization default:
- Persist only metadata, counters, and derived signals by default.
- Pane output is never stored unless the user explicitly enables capture for a session.
- Even when capture is enabled, the UI should default to showing only a redacted, bounded preview.

Redaction guarantees:
- Redaction must run before any output bytes are written to disk (preview cache, stream logs, diagnostics bundle).
- Apply redaction again at export time as a defense-in-depth layer.
- Keep redaction patterns configurable, but ship conservative defaults (cloud keys, private keys, common tokens).

Diagnostics bundle contents (default):
- Versions, platform details (Windows version, WSL distro), daemon config (with secrets removed).
- Recent structured logs (redacted) and health counters.
- DB summary (row counts, retention settings, schema version); do not include raw pane output.
- Optional: a short redacted output preview only if the user explicitly checks a box during export.

This section is primarily about making the privacy contract auditable and testable.

---

## 8. Risk Assessment & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| WSL2 networking issues | Medium | High | stdio default via `wsl.exe`; TCP/WS optional with clear fallback |
| Local API readable by other local processes | Medium | High | Require bearer token; request limits; redact sensitive output by default |
| Sensitive output captured (secrets) | Medium | High | Default capture=off; explicit per-session opt-in; redaction at ingestion + at export; clear UI indicators |
| Redaction bypass (sensitive data leakage) | Medium | High | Redact before persistence; defense-in-depth redaction at export; tests with seeded secrets corpus |
| Diagnostics bundle leaks sensitive data | Low | High | Bundle excludes raw output by default; explicit user opt-in for previews; strong redaction + size bounds |
| ntm output format changes | Low | High | Abstract parser, version detection |
| High CPU from frequent polling | Medium | Medium | Adaptive polling, efficient diffing |
| Hung tmux/ntm commands stall loops | Low | High | Hard timeouts; circuit breaker; surface degraded state without mutating DB |
| Missed compact events | Medium | Medium | Multiple detection methods, pattern tuning |
| Windows firewall blocks | Low | Medium | Documentation, localhost should be fine |
| Tmux permission issues | Low | Medium | Document user requirements |

---

## 9. Future Enhancements (Post-MVP)

- [ ] **Multi-machine support**: Track ntm on remote servers via SSH
- [ ] **tmux-only mode**: Monitor tmux sessions/panes even when NTM is unavailable; degrade compact detection gracefully
- [ ] **Historical analysis**: "Last week you averaged 12 compacts/day"
- [ ] **Cost estimation**: Estimate API costs based on token usage
- [ ] **Session templates**: Quick-launch common session configs
- [ ] **Export/reporting**: Daily/weekly PDF reports
- [ ] **CLI companion**: `ntm-tracker status`, `ntm-tracker health`, `ntm-tracker events --tail`
- [ ] **Detector packs**: versioned pattern bundles with golden tests; update independently of daemon binary
- [ ] **Integration with Clawdbot**: Surface metrics in chat

---

## 10. Open Questions

1. **Token counting accuracy**: Should we try to parse actual token counts from Claude output, or is estimation sufficient?

2. **Session persistence**: When ntm session ends, archive immediately or keep showing for X minutes?

3. **Multiple WSL distros**: Support tracking across different distros?

4. **Shared access**: Should the daemon support multiple UI clients connecting?

---

## Appendix A: NTM Command Reference

```bash
# List all sessions
ntm list
ntm --robot-markdown --md-compact

# Session details
ntm status <session>

# Pane output
ntm --robot-tail <session> --lines 50 --json

# Send to pane
ntm --robot-send <session> --panes <n> --msg "text" --json

# Kill session
ntm kill <session>
```

## Appendix B: Tmux Commands Used

```bash
# List sessions
tmux list-sessions -F '#{session_name}:#{session_windows}'

# List panes (include metadata for status detection)
tmux list-panes -a -F '#{session_id}:#{window_id}:#{pane_id}:#{pane_index}:#{pane_pid}:#{pane_current_command}:#{pane_last_activity}:#{pane_dead}:#{pane_in_mode}'

# Capture pane content
tmux capture-pane -t <session>:<pane> -p -S -1000

# Stream pane output incrementally (preferred)
tmux pipe-pane -t <session>:<pane> -o "cat >> ~/.local/share/ntm-tracker/panes/<pane_id>.log"
```
