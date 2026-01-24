use rusqlite::{Connection, OptionalExtension, Transaction};
use std::path::Path;

const SCHEMA_VERSION_KEY: &str = "schema_version";

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("migrations/0001_init.sql"),
}];

pub fn open_database(path: impl AsRef<Path>) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    apply_pragmas(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

pub fn migrate(conn: &mut Connection) -> rusqlite::Result<u32> {
    ensure_meta_table(conn)?;
    let current_version = read_schema_version(conn)?;

    for migration in MIGRATIONS {
        if migration.version > current_version {
            apply_migration(conn, migration)?;
        }
    }

    Ok(latest_version())
}

fn latest_version() -> u32 {
    MIGRATIONS
        .last()
        .map(|migration| migration.version)
        .unwrap_or(0)
}

fn apply_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        "#,
    )?;
    Ok(())
}

fn ensure_meta_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

fn read_schema_version(conn: &Connection) -> rusqlite::Result<u32> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1;",
            [SCHEMA_VERSION_KEY],
            |row| row.get(0),
        )
        .optional()?;

    Ok(value
        .as_deref()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(0))
}

fn apply_migration(conn: &mut Connection, migration: &Migration) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(migration.sql)?;
    write_schema_version(&tx, migration.version)?;
    tx.commit()?;
    Ok(())
}

fn write_schema_version(tx: &Transaction<'_>, version: u32) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2);",
        (SCHEMA_VERSION_KEY, version.to_string()),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        migrate(&mut conn).expect("initial migration");
        migrate(&mut conn).expect("repeat migration");

        let version = read_schema_version(&conn).expect("schema version");
        assert_eq!(version, latest_version());

        let table_exists: Option<String> = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'sessions';",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("query schema");
        assert_eq!(table_exists.as_deref(), Some("sessions"));
    }
}
