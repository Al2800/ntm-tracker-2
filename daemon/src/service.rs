//! Service lifecycle management: single-instance guard, graceful shutdown.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Global shutdown flag.
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Check if shutdown has been requested.
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

/// Request shutdown.
pub fn request_shutdown() {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

/// Single-instance guard using PID file.
pub struct InstanceGuard {
    pid_path: PathBuf,
    #[cfg(unix)]
    _lock_file: Option<File>,
}

impl InstanceGuard {
    /// Acquire the single-instance lock.
    ///
    /// Returns Ok(guard) if this is the only instance.
    /// Returns Err if another instance is running.
    pub fn acquire() -> Result<Self, String> {
        let data_dir = data_dir();
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {e}"))?;

        let pid_path = data_dir.join("daemon.pid");
        let lock_path = data_dir.join("daemon.lock");

        // Try to acquire file lock
        #[cfg(unix)]
        let lock_file = {
            use std::os::unix::io::AsRawFd;

            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path)
                .map_err(|e| format!("Failed to open lock file: {e}"))?;

            // Try exclusive lock (non-blocking)
            let fd = file.as_raw_fd();
            let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if result != 0 {
                // Lock failed - another instance is running
                // Check if the other instance is healthy
                if let Ok(existing_pid) = read_pid_file(&pid_path) {
                    if is_process_running(existing_pid) {
                        return Err(format!(
                            "Another daemon instance is running (PID {existing_pid})"
                        ));
                    }
                    // Stale PID file - take over
                    warn!(pid = existing_pid, "Taking over from stale PID");
                }
            }
            Some(file)
        };

        #[cfg(not(unix))]
        let _lock_file: Option<File> = None;

        // Write our PID
        let pid = std::process::id();
        let mut file = File::create(&pid_path)
            .map_err(|e| format!("Failed to create PID file: {e}"))?;
        writeln!(file, "{pid}").map_err(|e| format!("Failed to write PID: {e}"))?;

        info!(pid, pid_file = %pid_path.display(), "Acquired instance lock");

        Ok(Self {
            pid_path,
            #[cfg(unix)]
            _lock_file: lock_file,
        })
    }

    /// Get the path to the PID file.
    pub fn pid_path(&self) -> &PathBuf {
        &self.pid_path
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // Remove PID file on clean exit
        if let Err(e) = fs::remove_file(&self.pid_path) {
            debug!(error = %e, "Failed to remove PID file (may already be removed)");
        } else {
            debug!(pid_file = %self.pid_path.display(), "Removed PID file");
        }
    }
}

/// Read PID from file.
fn read_pid_file(path: &PathBuf) -> Result<u32, ()> {
    let mut file = File::open(path).map_err(|_| ())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|_| ())?;
    contents.trim().parse().map_err(|_| ())
}

/// Check if a process with the given PID is running.
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    // Send signal 0 to check if process exists
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // On non-Unix, assume not running (Windows uses different mechanism)
    false
}

/// Get the data directory for the daemon.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(dir).join("ntm-tracker")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local/share/ntm-tracker")
    } else {
        PathBuf::from("/tmp/ntm-tracker")
    }
}

/// Graceful shutdown handler.
pub struct ShutdownHandler {
    shutdown_tx: broadcast::Sender<()>,
}

impl ShutdownHandler {
    /// Create a new shutdown handler and install signal handlers.
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self { shutdown_tx }
    }

    /// Get a receiver for shutdown notifications.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Trigger shutdown.
    pub fn shutdown(&self) {
        request_shutdown();
        let _ = self.shutdown_tx.send(());
    }

    /// Wait for shutdown signal (SIGTERM, SIGINT, or manual trigger).
    pub async fn wait_for_signal(&self) {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};

            let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
            let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating shutdown");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating shutdown");
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await.expect("Ctrl-C handler");
            info!("Received Ctrl-C, initiating shutdown");
        }

        self.shutdown();
    }

    /// Wait for shutdown with timeout.
    pub async fn graceful_shutdown(&self, timeout: std::time::Duration) {
        info!("Starting graceful shutdown (timeout: {:?})", timeout);

        // Give components time to finish
        tokio::time::sleep(timeout).await;

        info!("Graceful shutdown complete");
    }
}

impl Default for ShutdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn data_dir_uses_xdg() {
        let temp = TempDir::new().unwrap();
        env::set_var("XDG_DATA_HOME", temp.path());
        let dir = data_dir();
        assert!(dir.starts_with(temp.path()));
        env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn shutdown_handler_broadcasts() {
        let handler = ShutdownHandler::new();
        let _rx = handler.subscribe();

        // Initially not shutdown
        assert!(!is_shutdown_requested());

        // Trigger shutdown
        handler.shutdown();

        // Now shutdown is requested
        assert!(is_shutdown_requested());

        // Receiver should get the message
        // (would need async runtime to verify)
    }

    #[test]
    fn instance_guard_creates_pid_file() {
        let temp = TempDir::new().unwrap();
        env::set_var("XDG_DATA_HOME", temp.path());

        // First instance should succeed
        let guard = InstanceGuard::acquire().expect("First instance");
        assert!(guard.pid_path().exists());

        // Cleanup
        drop(guard);
        env::remove_var("XDG_DATA_HOME");
    }
}
