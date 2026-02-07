pub mod fixtures;
pub mod logging;

use ntm_tracker_tui::msg::{ConnState, Msg};
use ntm_tracker_tui::rpc::types::JsonRpcMessage;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

/// Integration test harness that spawns the real daemon process.
pub struct TestHarness {
    pub child: Child,
    pub stdin_tx: mpsc::Sender<String>,
    pub msg_rx: mpsc::UnboundedReceiver<Msg>,
    pub logger: logging::TestLogger,
}

impl TestHarness {
    /// Spawn the daemon and start reader/writer tasks.
    pub async fn spawn(test_name: &str) -> Result<Self, String> {
        let logger = logging::TestLogger::new(test_name);
        logger.step("Spawning daemon process");

        let daemon_bin = find_daemon_binary();
        logger.log(&format!("Using daemon binary: {daemon_bin}"));

        let mut child = Command::new(&daemon_bin)
            .arg("start")
            .arg("--stdio")
            .arg("--log-level")
            .arg("error")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn daemon: {e}"))?;

        let stdin = child.stdin.take().ok_or("No stdin")?;
        let stdout = child.stdout.take().ok_or("No stdout")?;

        logger.step_result(true, "Daemon process spawned");

        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<Msg>();

        // Writer task
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(64);
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(line) = stdin_rx.recv().await {
                let _ = stdin.write_all(line.as_bytes()).await;
                let _ = stdin.write_all(b"\n").await;
                let _ = stdin.flush().await;
            }
        });

        // Reader task
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                let message: JsonRpcMessage = match serde_json::from_str(&line) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if message.is_notification() {
                    let method = message.method.as_deref().unwrap_or("");
                    match method {
                        "core.hello" => {
                            let version = message
                                .params
                                .as_ref()
                                .and_then(|p| p.get("daemonVersion"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let _ = msg_tx.send(Msg::HelloReceived(version));
                            let _ = msg_tx.send(Msg::ConnectionChanged(ConnState::Connected));
                        }
                        "sessions.snapshot" => {
                            if let Some(params) = &message.params {
                                if let Ok(snap) = serde_json::from_value(params.clone()) {
                                    let _ = msg_tx.send(Msg::SnapshotReceived(snap));
                                }
                            }
                        }
                        _ => {}
                    }
                } else if let Some(id_val) = &message.id {
                    // For now, forward as-is via a generic message.
                    // Individual tests can handle responses.
                    let _ = id_val; // suppress warning
                }
            }
            let _ = msg_tx.send(Msg::ConnectionChanged(ConnState::Disconnected));
        });

        Ok(TestHarness {
            child,
            stdin_tx,
            msg_rx,
            logger,
        })
    }

    /// Wait for a specific message predicate, with timeout.
    pub async fn wait_for_msg<F>(&mut self, timeout: Duration, predicate: F) -> Option<Msg>
    where
        F: Fn(&Msg) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            tokio::select! {
                msg = self.msg_rx.recv() => {
                    match msg {
                        Some(m) if predicate(&m) => return Some(m),
                        Some(_) => continue,
                        None => return None,
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    return None;
                }
            }
        }
    }

    /// Wait for HelloReceived message.
    pub async fn wait_for_hello(&mut self, timeout: Duration) -> Option<String> {
        let msg = self
            .wait_for_msg(timeout, |m| matches!(m, Msg::HelloReceived(_)))
            .await;
        match msg {
            Some(Msg::HelloReceived(v)) => Some(v),
            _ => None,
        }
    }

    /// Wait for a SnapshotReceived message.
    pub async fn wait_for_snapshot(
        &mut self,
        timeout: Duration,
    ) -> Option<ntm_tracker_tui::rpc::types::Snapshot> {
        let msg = self
            .wait_for_msg(timeout, |m| matches!(m, Msg::SnapshotReceived(_)))
            .await;
        match msg {
            Some(Msg::SnapshotReceived(s)) => Some(s),
            _ => None,
        }
    }

    /// Wait for ConnectionChanged to a specific state.
    pub async fn wait_for_connection_state(
        &mut self,
        state: ConnState,
        timeout: Duration,
    ) -> bool {
        let msg = self
            .wait_for_msg(timeout, |m| match m {
                Msg::ConnectionChanged(s) => *s == state,
                _ => false,
            })
            .await;
        msg.is_some()
    }

    /// Send a JSON-RPC request string to the daemon.
    pub async fn send_raw(&self, json: &str) -> Result<(), String> {
        self.stdin_tx
            .send(json.to_string())
            .await
            .map_err(|e| format!("send failed: {e}"))
    }

    /// Kill the daemon process.
    pub async fn shutdown(&mut self) {
        self.logger.step("Shutting down daemon");
        let _ = self.child.kill().await;
        self.logger.step_result(true, "Daemon process killed");
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // kill_on_drop handles cleanup
    }
}

/// Find the daemon binary — check common locations.
fn find_daemon_binary() -> String {
    let candidates = [
        "ntm-tracker-daemon",
        "/usr/local/bin/ntm-tracker-daemon",
        "../daemon/target/debug/ntm-tracker-daemon",
        "../target/debug/ntm-tracker-daemon",
    ];
    for candidate in &candidates {
        if std::process::Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return candidate.to_string();
        }
    }
    // Fallback — let it fail at spawn time with a clear error
    "ntm-tracker-daemon".to_string()
}
