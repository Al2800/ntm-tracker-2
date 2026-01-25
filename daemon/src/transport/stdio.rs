//! stdio transport adapter using newline-delimited JSON-RPC.
//!
//! This is the default transport when the daemon is spawned via `wsl.exe`.
//! It provides full duplex communication over stdin/stdout.

use crate::rpc::{self, RpcContext};
use crate::transport::{JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Run the stdio transport, processing requests from stdin and writing responses to stdout.
///
/// This function runs until stdin is closed or a fatal error occurs.
pub async fn run(ctx: Arc<RpcContext>, mut notification_rx: mpsc::Receiver<JsonRpcNotification>) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = stdout;

    info!("stdio transport started");

    loop {
        tokio::select! {
            // Handle incoming requests from stdin
            line_result = lines.next_line() => {
                match line_result {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        trace!(line = %line, "received request");
                        if let Some(response) = process_line(&line, &ctx) {
                            if let Err(e) = write_response(&mut stdout, &response).await {
                                error!(error = %e, "failed to write response");
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        info!("stdin closed, shutting down");
                        break;
                    }
                    Err(e) => {
                        error!(error = %e, "error reading stdin");
                        break;
                    }
                }
            }
            // Handle outgoing notifications
            Some(notification) = notification_rx.recv() => {
                if let Err(e) = write_notification(&mut stdout, &notification).await {
                    error!(error = %e, "failed to write notification");
                    break;
                }
            }
        }
    }

    info!("stdio transport stopped");
}

/// Process a single line of input and return a response if needed.
fn process_line(line: &str, ctx: &RpcContext) -> Option<JsonRpcResponse> {
    // Parse JSON
    let request: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            warn!(error = %e, "failed to parse JSON");
            return Some(JsonRpcResponse::error(
                Value::Null,
                JsonRpcError::parse_error(format!("Invalid JSON: {e}")),
            ));
        }
    };

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        warn!(version = %request.jsonrpc, "invalid JSON-RPC version");
        return Some(JsonRpcResponse::error(
            request.id.unwrap_or(Value::Null),
            JsonRpcError::invalid_request("Expected jsonrpc: \"2.0\""),
        ));
    }

    // If no id, this is a notification - no response expected
    let id = match request.id {
        Some(id) => id,
        None => {
            debug!(method = %request.method, "received notification (no response)");
            // Process notification but don't respond
            let _ = rpc::handle(&request.method, request.params, ctx);
            return None;
        }
    };

    // Handle the request
    debug!(method = %request.method, "handling request");
    let result = rpc::handle(&request.method, request.params, ctx);

    Some(match result {
        Ok(value) => JsonRpcResponse::success(id, value),
        Err(e) => JsonRpcResponse::error(id, JsonRpcError::from_rpc_error(&e)),
    })
}

/// Write a response to stdout as newline-delimited JSON.
async fn write_response(
    stdout: &mut tokio::io::Stdout,
    response: &JsonRpcResponse,
) -> std::io::Result<()> {
    let json = serde_json::to_string(response)?;
    trace!(response = %json, "sending response");
    stdout.write_all(json.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await
}

/// Write a notification to stdout as newline-delimited JSON.
async fn write_notification(
    stdout: &mut tokio::io::Stdout,
    notification: &JsonRpcNotification,
) -> std::io::Result<()> {
    let json = serde_json::to_string(notification)?;
    trace!(notification = %json, "sending notification");
    stdout.write_all(json.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await
}

/// Create a notification sender that can be used to push events to the client.
pub fn notification_channel() -> (mpsc::Sender<JsonRpcNotification>, mpsc::Receiver<JsonRpcNotification>) {
    mpsc::channel(256)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::ConfigManager;
    use std::sync::Arc;

    fn test_context() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        RpcContext::new(cache, config)
    }

    #[test]
    fn process_valid_request() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"health.get","params":{},"id":1}"#;
        let response = process_line(line, &ctx);
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn process_invalid_json() {
        let ctx = test_context();
        let line = "not valid json";
        let response = process_line(line, &ctx);
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, JsonRpcError::PARSE_ERROR);
    }

    #[test]
    fn process_invalid_version() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"1.0","method":"health.get","id":1}"#;
        let response = process_line(line, &ctx);
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, JsonRpcError::INVALID_REQUEST);
    }

    #[test]
    fn process_notification_no_response() {
        let ctx = test_context();
        // No "id" field means this is a notification
        let line = r#"{"jsonrpc":"2.0","method":"ping","params":{}}"#;
        let response = process_line(line, &ctx);
        assert!(response.is_none());
    }

    #[test]
    fn process_unknown_method() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"unknown.method","params":{},"id":1}"#;
        let response = process_line(line, &ctx);
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.error.is_some());
    }
}
