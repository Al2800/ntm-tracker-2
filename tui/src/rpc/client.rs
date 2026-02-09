use crate::msg::{ConnState, Msg};
use crate::rpc::types::{JsonRpcMessage, JsonRpcRequest, Snapshot};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{debug, error, info, warn};

/// Pending request waiting for a response.
type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

/// JSON-RPC client that communicates with the daemon over stdio.
pub struct RpcClient {
    /// Channel to send serialized JSON lines to the writer task.
    write_tx: mpsc::Sender<String>,
    /// Next request ID.
    next_id: AtomicU64,
    /// Pending request map.
    pending: PendingMap,
    /// Daemon child process handle.
    _child: Child,
}

impl RpcClient {
    /// Spawn the daemon and start the reader/writer tasks.
    ///
    /// `msg_tx` receives messages to feed into the TUI update loop.
    pub fn spawn(daemon_bin: &str, msg_tx: mpsc::UnboundedSender<Msg>) -> Result<Self, String> {
        let mut child = Command::new(daemon_bin)
            .arg("start")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn daemon: {e}"))?;

        let stdin = child.stdin.take().ok_or("No stdin on daemon process")?;
        let stdout = child.stdout.take().ok_or("No stdout on daemon process")?;

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

        // Writer task: sends lines to daemon stdin.
        let (write_tx, mut write_rx) = mpsc::channel::<String>(64);
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(line) = write_rx.recv().await {
                if let Err(e) = stdin.write_all(line.as_bytes()).await {
                    error!("stdin write error: {e}");
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    error!("stdin newline error: {e}");
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("stdin flush error: {e}");
                    break;
                }
            }
            debug!("writer task ended");
        });

        // Reader task: reads lines from daemon stdout.
        let pending_clone = pending.clone();
        let msg_tx_reader = msg_tx.clone();
        tokio::spawn(async move {
            let msg_tx = msg_tx_reader;
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                let message: JsonRpcMessage = match serde_json::from_str(&line) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("Failed to parse daemon message: {e}");
                        continue;
                    }
                };

                if message.is_notification() {
                    handle_notification(&message, &msg_tx);
                } else if let Some(id_val) = &message.id {
                    // Response to a pending request.
                    let id = id_val.as_u64().unwrap_or(0);
                    let mut pending = pending_clone.lock().await;
                    if let Some(tx) = pending.remove(&id) {
                        if let Some(err) = &message.error {
                            let _ = tx.send(Err(err.message.clone()));
                        } else {
                            let _ = tx.send(Ok(message.result.unwrap_or(Value::Null)));
                        }
                    }
                }
            }

            info!("reader task ended — daemon stdout closed");
            let _ = msg_tx.send(Msg::ConnectionChanged(ConnState::Disconnected));
        });

        let _ = msg_tx.send(Msg::ConnectionChanged(ConnState::Connecting));

        Ok(Self {
            write_tx,
            next_id: AtomicU64::new(1),
            pending,
            _child: child,
        })
    }

    /// Send a request and return a oneshot receiver for the result.
    pub async fn request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<oneshot::Receiver<Result<Value, String>>, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = JsonRpcRequest::new(id, method, params);
        let json = serde_json::to_string(&req).map_err(|e| e.to_string())?;

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        self.write_tx
            .send(json)
            .await
            .map_err(|e| format!("write channel closed: {e}"))?;

        Ok(rx)
    }

    /// Convenience: send snapshot.get and deserialize.
    pub async fn get_snapshot(&self) -> Result<oneshot::Receiver<Result<Value, String>>, String> {
        self.request("snapshot.get", Value::Null).await
    }

    /// Clone the write channel sender for fire-and-forget notifications.
    pub fn write_sender(&self) -> mpsc::Sender<String> {
        self.write_tx.clone()
    }
}

fn handle_notification(msg: &JsonRpcMessage, tx: &mpsc::UnboundedSender<Msg>) {
    let method = msg.method.as_deref().unwrap_or("");
    match method {
        "core.hello" => {
            let version = msg
                .params
                .as_ref()
                .and_then(|p| p.get("daemonVersion"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let _ = tx.send(Msg::HelloReceived(version));
            let _ = tx.send(Msg::ConnectionChanged(ConnState::Connected));
        }
        "sessions.snapshot" => {
            if let Some(params) = &msg.params {
                match serde_json::from_value::<Snapshot>(params.clone()) {
                    Ok(snap) => {
                        let _ = tx.send(Msg::SnapshotReceived(snap));
                    }
                    Err(e) => {
                        warn!("Failed to parse snapshot notification: {e}");
                    }
                }
            }
        }
        _ => {
            debug!("Unhandled notification: {method}");
        }
    }
}
