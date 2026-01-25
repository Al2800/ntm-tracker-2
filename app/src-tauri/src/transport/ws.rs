use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tungstenite::{connect, Message};
use url::Url;

#[derive(Debug)]
pub struct WsTransport {
    url: Url,
    next_id: AtomicU64,
}

impl WsTransport {
    pub fn new(url: &str) -> Result<Self, String> {
        let url = Url::parse(url).map_err(|err| format!("Invalid WS url: {err}"))?;
        Ok(Self {
            url,
            next_id: AtomicU64::new(1),
        })
    }

    pub fn call(&self, method: String, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let payload = serde_json::to_string(&request)
            .map_err(|err| format!("Failed to serialize JSON-RPC request: {err}"))?;

        let (mut socket, _) =
            connect(self.url.clone()).map_err(|err| format!("WS connect failed: {err}"))?;

        socket
            .send(Message::Text(payload))
            .map_err(|err| format!("WS send failed: {err}"))?;

        let started = Instant::now();
        loop {
            if started.elapsed() > timeout {
                return Err("WS RPC call timed out".to_string());
            }

            let msg = socket
                .read()
                .map_err(|err| format!("WS read failed: {err}"))?;

            let text = match msg {
                Message::Text(text) => text,
                Message::Binary(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                _ => continue,
            };

            let value: Value = serde_json::from_str(&text)
                .map_err(|err| format!("Invalid JSON-RPC response: {err}"))?;

            if value.get("id").and_then(|id| id.as_u64()) != Some(id) {
                continue;
            }

            if let Some(error) = value.get("error") {
                return Err(error.to_string());
            }

            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}
