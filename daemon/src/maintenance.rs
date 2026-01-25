//! Database maintenance routines (rollups, retention, vacuum).

use crate::config::MaintenanceConfig;
use crate::db;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

const META_LAST_HOURLY: &str = "maintenance_last_hourly_rollup";
const META_LAST_DAILY: &str = "maintenance_last_daily_rollup";
const META_LAST_RETENTION: &str = "maintenance_last_retention";
const META_LAST_VACUUM: &str = "maintenance_last_vacuum";
const MAX_ROLLUP_HOURS_PER_RUN: i64 = 24;
const MAX_ROLLUP_DAYS_PER_RUN: i64 = 7;

#[derive(Debug, Default, Clone)]
pub struct RetentionSummary {
    pub minute_samples_deleted: usize,
    pub events_deleted: usize,
    pub sessions_archived: usize,
}

#[derive(Debug, Default, Clone)]
pub struct MaintenanceSummary {
    pub hours_rolled: usize,
    pub days_rolled: usize,
    pub retention: RetentionSummary,
    pub vacuum_ran: bool,
    pub db_size_mb: Option<u64>,
}

pub struct MaintenanceRunner {
    db_path: PathBuf,
    config: MaintenanceConfig,
    tz_offset_min: i64,
}

impl MaintenanceRunner {
    pub fn new(db_path: PathBuf, config: MaintenanceConfig) -> Self {
        Self {
            db_path,
            config,
            tz_offset_min: 0,
        }
    }

    pub fn run_once(&self) -> rusqlite::Result<MaintenanceSummary> {
        if !self.db_path.exists() {
            debug!(
                db_path = %self.db_path.display(),
                "Maintenance skipped (db missing)"
            );
            return Ok(MaintenanceSummary::default());
        }

        let mut conn = db::open_database(&self.db_path)?;
        let now = now_ts()?;
        run_cycle(
            &mut conn,
            &self.config,
            now,
            self.tz_offset_min,
            Some(&self.db_path),
        )
    }

    pub async fn run_loop(self, mut shutdown: broadcast::Receiver<()>) {
        let interval_ms = self.config.rollup_interval_ms.max(60_000);
        let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(err) = self.run_once() {
                        warn!(error = %err, "maintenance cycle failed");
                    }
                }
                _ = shutdown.recv() => {
                    info!("maintenance shutdown received");
                    break;
                }
            }
        }
    }
}

fn now_ts() -> rusqlite::Result<i64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    Ok(now.as_secs() as i64)
}

fn read_meta_i64(conn: &Connection, key: &str) -> rusqlite::Result<Option<i64>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = ?1;", [key], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(value.and_then(|raw| raw.parse::<i64>().ok()))
}

fn write_meta_i64(conn: &Connection, key: &str, value: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2);",
        params![key, value.to_string()],
    )?;
    Ok(())
}

fn latest_complete_hour(now: i64) -> Option<i64> {
    let current_hour = (now / 3600) * 3600;
    let latest_complete = current_hour - 3600;
    if latest_complete >= 0 {
        Some(latest_complete)
    } else {
        None
    }
}

fn latest_complete_day(now: i64) -> Option<i64> {
    let current_day = (now / 86_400) * 86_400;
    let latest_complete = current_day - 86_400;
    if latest_complete >= 0 {
        Some(latest_complete)
    } else {
        None
    }
}

pub fn run_cycle(
    conn: &mut Connection,
    config: &MaintenanceConfig,
    now: i64,
    tz_offset_min: i64,
    db_path: Option<&Path>,
) -> rusqlite::Result<MaintenanceSummary> {
    let mut summary = MaintenanceSummary::default();

    if let Some(latest_hour) = latest_complete_hour(now) {
        let last_hour = read_meta_i64(conn, META_LAST_HOURLY)?.unwrap_or(latest_hour - 3600);
        if latest_hour > last_hour {
            let mut hour = last_hour + 3600;
            let mut processed = 0;
            while hour <= latest_hour && processed < MAX_ROLLUP_HOURS_PER_RUN {
                rollup_hour(conn, hour)?;
                processed += 1;
                hour += 3600;
            }
            let last_processed = hour - 3600;
            write_meta_i64(conn, META_LAST_HOURLY, last_processed)?;
            summary.hours_rolled = processed as usize;
            info!(hours = summary.hours_rolled, "Hourly rollup complete");
        }
    }

    if let Some(latest_day) = latest_complete_day(now) {
        let last_day = read_meta_i64(conn, META_LAST_DAILY)?.unwrap_or(latest_day - 86_400);
        if latest_day > last_day {
            let mut day = last_day + 86_400;
            let mut processed = 0;
            while day <= latest_day && processed < MAX_ROLLUP_DAYS_PER_RUN {
                rollup_day(conn, day, tz_offset_min)?;
                processed += 1;
                day += 86_400;
            }
            let last_processed = day - 86_400;
            write_meta_i64(conn, META_LAST_DAILY, last_processed)?;
            summary.days_rolled = processed as usize;
            info!(days = summary.days_rolled, "Daily rollup complete");
        }
    }

    let last_retention = read_meta_i64(conn, META_LAST_RETENTION)?.unwrap_or(0);
    if now.saturating_sub(last_retention) >= 86_400 {
        summary.retention = enforce_retention(conn, now, config)?;
        write_meta_i64(conn, META_LAST_RETENTION, now)?;
    }

    let vacuum_interval = config.vacuum_interval_hours.saturating_mul(3600) as i64;
    let last_vacuum = read_meta_i64(conn, META_LAST_VACUUM)?.unwrap_or(0);
    if vacuum_interval > 0 && now.saturating_sub(last_vacuum) >= vacuum_interval {
        run_vacuum(conn)?;
        write_meta_i64(conn, META_LAST_VACUUM, now)?;
        summary.vacuum_ran = true;
    }

    if let Some(path) = db_path {
        if let Some(size_mb) = db_size_mb(path) {
            summary.db_size_mb = Some(size_mb);
            if config.max_db_mb > 0 && size_mb > config.max_db_mb {
                warn!(
                    db_size_mb = size_mb,
                    max_db_mb = config.max_db_mb,
                    "Database size exceeds limit; enforcing retention"
                );
                let retention = enforce_retention(conn, now, config)?;
                summary.retention.minute_samples_deleted += retention.minute_samples_deleted;
                summary.retention.events_deleted += retention.events_deleted;
                summary.retention.sessions_archived += retention.sessions_archived;
            }
        }
    }

    Ok(summary)
}

fn db_size_mb(path: &Path) -> Option<u64> {
    let bytes = std::fs::metadata(path).ok()?.len();
    Some((bytes / (1024 * 1024)).max(1))
}

pub fn rollup_hour(conn: &Connection, hour_start: i64) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO hourly_stats (hour_start, session_uid, total_compacts, active_minutes, estimated_tokens)
        SELECT
            ?1 as hour_start,
            panes.session_uid,
            0 as total_compacts,
            SUM(CASE WHEN pane_minute_samples.status IN ('active','waiting') THEN 1 ELSE 0 END) as active_minutes,
            SUM(pane_minute_samples.estimated_tokens) as estimated_tokens
        FROM pane_minute_samples
        JOIN panes ON panes.pane_uid = pane_minute_samples.pane_uid
        WHERE pane_minute_samples.minute_start >= ?1
          AND pane_minute_samples.minute_start < (?1 + 3600)
        GROUP BY panes.session_uid
        ON CONFLICT(hour_start, session_uid) DO UPDATE SET
          active_minutes = excluded.active_minutes,
          estimated_tokens = excluded.estimated_tokens;
        "#,
        [hour_start],
    )?;

    conn.execute(
        r#"
        UPDATE hourly_stats
        SET total_compacts = (
            SELECT COUNT(*)
            FROM events
            WHERE events.session_uid = hourly_stats.session_uid
              AND events.type = 'compact'
              AND events.detected_at >= ?1
              AND events.detected_at < (?1 + 3600)
        )
        WHERE hourly_stats.hour_start = ?1;
        "#,
        [hour_start],
    )?;

    Ok(())
}

pub fn rollup_day(conn: &Connection, day_start: i64, tz_offset_min: i64) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO daily_stats (day_start, tz_offset_min, session_uid, total_compacts, active_minutes, estimated_tokens)
        SELECT
            ?1 as day_start,
            ?2 as tz_offset_min,
            panes.session_uid,
            0 as total_compacts,
            SUM(CASE WHEN pane_minute_samples.status IN ('active','waiting') THEN 1 ELSE 0 END) as active_minutes,
            SUM(pane_minute_samples.estimated_tokens) as estimated_tokens
        FROM pane_minute_samples
        JOIN panes ON panes.pane_uid = pane_minute_samples.pane_uid
        WHERE pane_minute_samples.minute_start >= ?1
          AND pane_minute_samples.minute_start < (?1 + 86_400)
        GROUP BY panes.session_uid
        ON CONFLICT(day_start, session_uid) DO UPDATE SET
          tz_offset_min = excluded.tz_offset_min,
          active_minutes = excluded.active_minutes,
          estimated_tokens = excluded.estimated_tokens;
        "#,
        params![day_start, tz_offset_min],
    )?;

    conn.execute(
        r#"
        UPDATE daily_stats
        SET total_compacts = (
            SELECT COUNT(*)
            FROM events
            WHERE events.session_uid = daily_stats.session_uid
              AND events.type = 'compact'
              AND events.detected_at >= ?1
              AND events.detected_at < (?1 + 86_400)
        )
        WHERE daily_stats.day_start = ?1;
        "#,
        [day_start],
    )?;

    Ok(())
}

pub fn enforce_retention(
    conn: &Connection,
    now: i64,
    config: &MaintenanceConfig,
) -> rusqlite::Result<RetentionSummary> {
    let minute_cutoff = now.saturating_sub((config.minute_samples_retention_hours as i64) * 3600);
    let event_cutoff = now.saturating_sub((config.events_retention_days as i64) * 86_400);
    let session_cutoff = now.saturating_sub((config.sessions_retention_days as i64) * 86_400);

    let minute_samples_deleted = conn.execute(
        "DELETE FROM pane_minute_samples WHERE minute_start < ?1;",
        [minute_cutoff],
    )?;

    let events_deleted = conn.execute(
        "DELETE FROM events WHERE detected_at < ?1;",
        [event_cutoff],
    )?;

    let sessions_archived = conn.execute(
        "UPDATE sessions SET status = 'ended', status_reason = 'archived'
         WHERE ended_at IS NOT NULL AND ended_at < ?1;",
        [session_cutoff],
    )?;

    info!(
        minute_samples_deleted,
        events_deleted,
        sessions_archived,
        "Retention enforcement complete"
    );

    Ok(RetentionSummary {
        minute_samples_deleted,
        events_deleted,
        sessions_archived,
    })
}

pub fn run_vacuum(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA wal_checkpoint(TRUNCATE);
        PRAGMA incremental_vacuum;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup_session(conn: &Connection, session_uid: &str, pane_uid: &str) {
        conn.execute(
            "INSERT INTO sources (source_id, kind, distro, tmux_socket, created_at, last_seen_at, status, last_error, metadata)
             VALUES (?1, 'ntm', 'ubuntu', NULL, 0, 0, 'ok', NULL, NULL);",
            params!["src-1"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (session_uid, source_id, tmux_session_id, name, created_at, last_seen_at, ended_at, status, status_reason, pane_count, metadata)
             VALUES (?1, 'src-1', NULL, 'sess', 0, 0, NULL, 'active', NULL, 0, NULL);",
            params![session_uid],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO panes (pane_uid, session_uid, tmux_pane_id, tmux_window_id, tmux_pane_pid, pane_index, agent_type, created_at, last_seen_at, last_activity_at, current_command, ended_at, status, status_reason)
             VALUES (?1, ?2, NULL, NULL, NULL, 0, NULL, 0, 0, NULL, NULL, NULL, 'active', NULL);",
            params![pane_uid, session_uid],
        )
        .unwrap();
    }

    #[test]
    fn rollup_hour_creates_stats() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();

        setup_session(&conn, "sess-1", "pane-1");

        conn.execute(
            "INSERT INTO pane_minute_samples (minute_start, pane_uid, status, output_lines, output_bytes, estimated_tokens)
             VALUES (?1, ?2, 'active', 1, 10, 100);",
            params![7200, "pane-1"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pane_minute_samples (minute_start, pane_uid, status, output_lines, output_bytes, estimated_tokens)
             VALUES (?1, ?2, 'active', 2, 20, 200);",
            params![7260, "pane-1"],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO events (session_uid, pane_uid, type, detected_at, source, confidence, severity, status, resolved_at, trigger, message, context_before, payload, dedupe_hash)
             VALUES (?1, ?2, 'compact', ?3, 'auto', 1.0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);",
            params!["sess-1", "pane-1", 7300],
        )
        .unwrap();

        rollup_hour(&conn, 7200).unwrap();

        let (active_minutes, estimated_tokens, total_compacts): (i64, i64, i64) = conn
            .query_row(
                "SELECT active_minutes, estimated_tokens, total_compacts FROM hourly_stats WHERE hour_start = 7200 AND session_uid = 'sess-1';",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(active_minutes, 2);
        assert_eq!(estimated_tokens, 300);
        assert_eq!(total_compacts, 1);
    }

    #[test]
    fn rollup_day_creates_stats() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();

        setup_session(&conn, "sess-2", "pane-2");

        conn.execute(
            "INSERT INTO pane_minute_samples (minute_start, pane_uid, status, output_lines, output_bytes, estimated_tokens)
             VALUES (?1, ?2, 'active', 1, 10, 150);",
            params![86_460, "pane-2"],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO events (session_uid, pane_uid, type, detected_at, source, confidence, severity, status, resolved_at, trigger, message, context_before, payload, dedupe_hash)
             VALUES (?1, ?2, 'compact', ?3, 'auto', 1.0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);",
            params!["sess-2", "pane-2", 86_500],
        )
        .unwrap();

        rollup_day(&conn, 86_400, 0).unwrap();

        let (active_minutes, estimated_tokens, total_compacts): (i64, i64, i64) = conn
            .query_row(
                "SELECT active_minutes, estimated_tokens, total_compacts FROM daily_stats WHERE day_start = 86400 AND session_uid = 'sess-2';",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(active_minutes, 1);
        assert_eq!(estimated_tokens, 150);
        assert_eq!(total_compacts, 1);
    }

    #[test]
    fn retention_prunes_old_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();

        setup_session(&conn, "sess-3", "pane-3");

        conn.execute(
            "INSERT INTO pane_minute_samples (minute_start, pane_uid, status, output_lines, output_bytes, estimated_tokens)
             VALUES (?1, ?2, 'active', 1, 10, 100);",
            params![1000, "pane-3"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (session_uid, pane_uid, type, detected_at, source, confidence, severity, status, resolved_at, trigger, message, context_before, payload, dedupe_hash)
             VALUES (?1, ?2, 'compact', ?3, 'auto', 1.0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);",
            params!["sess-3", "pane-3", 1000],
        )
        .unwrap();
        conn.execute(
            "UPDATE sessions SET ended_at = ?1 WHERE session_uid = 'sess-3';",
            params![1000],
        )
        .unwrap();

        let config = MaintenanceConfig {
            minute_samples_retention_hours: 1,
            events_retention_days: 1,
            sessions_retention_days: 1,
            ..MaintenanceConfig::default()
        };

        let summary = enforce_retention(&conn, 200_000, &config).unwrap();

        assert_eq!(summary.minute_samples_deleted, 1);
        assert_eq!(summary.events_deleted, 1);
        assert_eq!(summary.sessions_archived, 1);

        let status_reason: Option<String> = conn
            .query_row(
                "SELECT status_reason FROM sessions WHERE session_uid = 'sess-3';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status_reason.as_deref(), Some("archived"));
    }
}
