use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub type NotificationHandler = Arc<dyn Fn(Value) + Send + Sync>;

#[derive(Debug)]
pub struct StdioTransport {
    child: Mutex<Child>,
    stdin: Mutex<BufWriter<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, mpsc::Sender<Result<Value, String>>>>>,
    next_id: AtomicU64,
}

impl StdioTransport {
    pub fn spawn(
        mut command: Command,
        notification_handler: Option<NotificationHandler>,
    ) -> Result<Self, String> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = command
            .spawn()
            .map_err(|err| format!("Failed to spawn daemon process: {err}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Unable to take daemon stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Unable to take daemon stdout".to_string())?;

        let pending: Arc<Mutex<HashMap<u64, mpsc::Sender<Result<Value, String>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        start_stdout_reader(stdout, pending.clone(), notification_handler);

        Ok(Self {
            child: Mutex::new(child),
            stdin: Mutex::new(BufWriter::new(stdin)),
            pending,
            next_id: AtomicU64::new(1),
        })
    }

    pub fn is_running(&self) -> bool {
        let mut child = match self.child.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        child.try_wait().ok().flatten().is_none()
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| "Daemon process lock poisoned".to_string())?;
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    }

    pub fn call(&self, method: String, params: Value, timeout: Duration) -> Result<Value, String> {
        if !self.is_running() {
            return Err("Daemon process is not running".to_string());
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let payload = serde_json::to_string(&request)
            .map_err(|err| format!("Failed to serialize JSON-RPC request: {err}"))?;

        let (tx, rx) = mpsc::channel();
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| "Pending request map lock poisoned".to_string())?;
            pending.insert(id, tx);
        }

        {
            let mut stdin = self
                .stdin
                .lock()
                .map_err(|_| "Daemon stdin lock poisoned".to_string())?;
            stdin
                .write_all(payload.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
                .and_then(|_| stdin.flush())
                .map_err(|err| format!("Failed to write JSON-RPC request: {err}"))?;
        }

        let started = Instant::now();
        loop {
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                remove_pending(&self.pending, id);
                return Err("RPC call timed out".to_string());
            }

            match rx.recv_timeout(remaining.min(Duration::from_millis(250))) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !self.is_running() {
                        remove_pending(&self.pending, id);
                        return Err("Daemon exited while waiting for RPC response".to_string());
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    remove_pending(&self.pending, id);
                    return Err("RPC response channel closed".to_string());
                }
            }
        }
    }
}

fn remove_pending(
    pending: &Arc<Mutex<HashMap<u64, mpsc::Sender<Result<Value, String>>>>>,
    id: u64,
) {
    if let Ok(mut guard) = pending.lock() {
        guard.remove(&id);
    }
}

fn start_stdout_reader(
    stdout: ChildStdout,
    pending: Arc<Mutex<HashMap<u64, mpsc::Sender<Result<Value, String>>>>>,
    notification_handler: Option<NotificationHandler>,
) {
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(err) => {
                    let _ = err;
                    continue;
                }
            };

            let Some(id) = value.get("id").and_then(|id| id.as_u64()) else {
                if let Some(handler) = notification_handler.as_ref() {
                    if let Some(method) = value.get("method").and_then(|method| method.as_str()) {
                        let params = value.get("params").cloned().unwrap_or(Value::Null);
                        let payload = json!({
                            "method": method,
                            "params": params,
                        });
                        handler(payload);
                    }
                }
                continue;
            };

            let tx = {
                let mut guard = match pending.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                guard.remove(&id)
            };

            let Some(tx) = tx else {
                continue;
            };

            if let Some(error) = value.get("error") {
                let _ = tx.send(Err(error.to_string()));
                continue;
            }
            let result = value.get("result").cloned().unwrap_or(Value::Null);
            let _ = tx.send(Ok(result));
        }
    });
}
