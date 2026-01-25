pub const APP_NAME: &str = "ntm-tracker-daemon";

pub mod bus;
pub mod cache;
pub mod cli;
pub mod collector;
pub mod command;
pub mod config;
pub mod db;
pub mod detector;
pub mod logging;
pub mod metrics;
pub mod models;
pub mod ntm;
pub mod parsers;
pub mod reconcile;
pub mod redaction;
pub mod rpc;
pub mod service;
pub mod state;
pub mod token_estimator;
pub mod transport;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn open_in_memory_db() -> rusqlite::Result<rusqlite::Connection> {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute("PRAGMA foreign_keys = ON;", [])?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn in_memory_db_opens() {
        let conn = open_in_memory_db().expect("open in-memory db");
        let result: i64 = conn.query_row("SELECT 1;", [], |row| row.get(0)).unwrap();
        assert_eq!(result, 1);
    }
}
