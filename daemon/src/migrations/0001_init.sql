-- Schema version 1: core tables and indexes
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS daemon_runs (
    run_id TEXT PRIMARY KEY,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    version TEXT NOT NULL,
    protocol_version INTEGER NOT NULL,
    schema_version INTEGER NOT NULL,
    capabilities TEXT
);

CREATE TABLE IF NOT EXISTS sources (
    source_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    distro TEXT NOT NULL,
    tmux_socket TEXT,
    created_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    status TEXT NOT NULL,
    last_error TEXT,
    metadata TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sources_unique
    ON sources(kind, distro, tmux_socket);
CREATE INDEX IF NOT EXISTS idx_sources_status
    ON sources(status);

CREATE TABLE IF NOT EXISTS sessions (
    session_uid TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    tmux_session_id TEXT,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    ended_at INTEGER,
    status TEXT NOT NULL,
    status_reason TEXT,
    pane_count INTEGER DEFAULT 0,
    metadata TEXT,
    FOREIGN KEY (source_id) REFERENCES sources(source_id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_name_active
    ON sessions(source_id, name) WHERE ended_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_sessions_status
    ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_source
    ON sessions(source_id);

CREATE TABLE IF NOT EXISTS panes (
    pane_uid TEXT PRIMARY KEY,
    session_uid TEXT NOT NULL,
    tmux_pane_id TEXT,
    tmux_window_id TEXT,
    tmux_pane_pid INTEGER,
    pane_index INTEGER NOT NULL,
    agent_type TEXT,
    created_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    last_activity_at INTEGER,
    current_command TEXT,
    ended_at INTEGER,
    status TEXT NOT NULL,
    status_reason TEXT,
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_panes_session
    ON panes(session_uid);
CREATE INDEX IF NOT EXISTS idx_panes_status
    ON panes(status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_panes_session_tmux
    ON panes(session_uid, tmux_pane_id)
    WHERE tmux_pane_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS pane_minute_samples (
    minute_start INTEGER NOT NULL,
    pane_uid TEXT NOT NULL,
    status TEXT NOT NULL,
    output_lines INTEGER DEFAULT 0,
    output_bytes INTEGER DEFAULT 0,
    estimated_tokens INTEGER DEFAULT 0,
    PRIMARY KEY (minute_start, pane_uid),
    FOREIGN KEY (pane_uid) REFERENCES panes(pane_uid) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pane_minute_time
    ON pane_minute_samples(minute_start);

CREATE TABLE IF NOT EXISTS hourly_stats (
    hour_start INTEGER NOT NULL,
    session_uid TEXT NOT NULL,
    total_compacts INTEGER DEFAULT 0,
    active_minutes INTEGER DEFAULT 0,
    estimated_tokens INTEGER DEFAULT 0,
    PRIMARY KEY (hour_start, session_uid),
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid)
);

CREATE TABLE IF NOT EXISTS daily_stats (
    day_start INTEGER NOT NULL,
    tz_offset_min INTEGER NOT NULL,
    session_uid TEXT NOT NULL,
    total_compacts INTEGER DEFAULT 0,
    active_minutes INTEGER DEFAULT 0,
    estimated_tokens INTEGER DEFAULT 0,
    PRIMARY KEY (day_start, session_uid),
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid)
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_uid TEXT NOT NULL,
    pane_uid TEXT NOT NULL,
    type TEXT NOT NULL,
    detected_at INTEGER NOT NULL,
    source TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    severity TEXT,
    status TEXT,
    resolved_at INTEGER,
    trigger TEXT,
    message TEXT,
    context_before INTEGER,
    payload TEXT,
    dedupe_hash TEXT,
    FOREIGN KEY (pane_uid) REFERENCES panes(pane_uid) ON DELETE CASCADE,
    FOREIGN KEY (session_uid) REFERENCES sessions(session_uid) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_hourly_time
    ON hourly_stats(hour_start);
CREATE INDEX IF NOT EXISTS idx_daily_time
    ON daily_stats(day_start);
CREATE INDEX IF NOT EXISTS idx_events_pane_time
    ON events(pane_uid, detected_at);
CREATE INDEX IF NOT EXISTS idx_events_type_time
    ON events(type, detected_at);
CREATE INDEX IF NOT EXISTS idx_events_source_time
    ON events(source, detected_at);
CREATE INDEX IF NOT EXISTS idx_events_status
    ON events(status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_events_dedupe
    ON events(dedupe_hash) WHERE dedupe_hash IS NOT NULL;
