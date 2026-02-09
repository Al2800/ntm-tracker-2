//! stdio transport adapter using newline-delimited JSON-RPC.
//!
//! This is the default transport when the daemon is spawned via `wsl.exe`.
//! It provides full duplex communication over stdin/stdout.

use crate::rpc::{self, RpcContext};
use crate::transport::{JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::metrics::{Timer, METRICS};
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

    let hello = JsonRpcNotification::new("core.hello", rpc::hello_payload(ctx.as_ref()));
    if let Err(e) = write_notification(&mut stdout, &hello).await {
        error!(error = %e, "failed to write hello notification");
        return;
    }

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
            let _timer = Timer::new(&METRICS.rpc_request);
            let _ = rpc::handle(&request.method, request.params, ctx);
            return None;
        }
    };

    // Handle the request
    debug!(method = %request.method, "handling request");
    let _timer = Timer::new(&METRICS.rpc_request);
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
    use crate::rpc::Capabilities;
    use std::sync::Arc;

    fn test_context() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let caps = Capabilities {
            ntm: false,
            tmux: true,
            stream: false,
            systemd: false,
        };
        RpcContext::with_capabilities(cache, config, caps)
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

    // --- New tests for bd-31f7 ---

    #[test]
    fn hello_notification_format() {
        let ctx = test_context();
        let hello = JsonRpcNotification::new("core.hello", rpc::hello_payload(&ctx));
        let json = serde_json::to_value(&hello).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "core.hello");
        assert!(json["params"]["daemonVersion"].is_string());
        assert_eq!(json["params"]["protocolVersion"], 1);
        assert_eq!(json["params"]["schemaVersion"], 1);
        assert!(json["params"]["instanceId"].is_string());
        assert!(json["params"]["runId"].is_string());
        assert_eq!(json["params"]["capabilities"]["tmux"], true);
        assert_eq!(json["params"]["capabilities"]["ntm"], false);
    }

    #[test]
    fn response_echoes_numeric_id() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"health.get","params":{},"id":42}"#;
        let resp = process_line(line, &ctx).unwrap();
        assert_eq!(resp.id, serde_json::json!(42));
    }

    #[test]
    fn response_echoes_string_id() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"health.get","params":{},"id":"req-abc"}"#;
        let resp = process_line(line, &ctx).unwrap();
        assert_eq!(resp.id, serde_json::json!("req-abc"));
    }

    #[test]
    fn error_response_echoes_id() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"no.such.method","params":{},"id":99}"#;
        let resp = process_line(line, &ctx).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.id, serde_json::json!(99));
    }

    #[test]
    fn parse_error_uses_null_id() {
        let ctx = test_context();
        let line = "{broken json";
        let resp = process_line(line, &ctx).unwrap();
        assert_eq!(resp.id, Value::Null);
        assert_eq!(resp.error.unwrap().code, JsonRpcError::PARSE_ERROR);
    }

    #[test]
    fn notification_channel_creates_bounded_channel() {
        let (tx, _rx) = notification_channel();
        // Channel should accept sends up to its capacity (256)
        assert!(!tx.is_closed());
    }

    #[test]
    fn process_sessions_list_returns_success() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"sessions.list","params":{},"id":1}"#;
        let resp = process_line(line, &ctx).unwrap();
        assert!(resp.result.is_some(), "sessions.list should return success");
        assert!(resp.error.is_none());
        // Result should contain a sessions array
        let result = resp.result.unwrap();
        assert!(result["sessions"].is_array());
    }

    #[test]
    fn process_snapshot_get_returns_success() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"snapshot.get","params":{},"id":1}"#;
        let resp = process_line(line, &ctx).unwrap();
        assert!(resp.result.is_some(), "snapshot.get should return success");
        assert!(resp.error.is_none());
    }

    #[test]
    fn process_stats_summary_returns_success() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"stats.summary","params":{},"id":1}"#;
        let resp = process_line(line, &ctx).unwrap();
        assert!(resp.result.is_some(), "stats.summary should return success");
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_serializes_to_valid_jsonrpc() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"health.get","params":{},"id":1}"#;
        let resp = process_line(line, &ctx).unwrap();
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert!(parsed.get("result").is_some());
        // Error field should not be present in success response
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn error_response_serializes_with_code_and_message() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"2.0","method":"no.such","params":{},"id":5}"#;
        let resp = process_line(line, &ctx).unwrap();
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 5);
        assert!(parsed["error"]["code"].is_number());
        assert!(parsed["error"]["message"].is_string());
        // Result field should not be present in error response
        assert!(parsed.get("result").is_none());
    }

    #[test]
    fn invalid_version_preserves_request_id() {
        let ctx = test_context();
        let line = r#"{"jsonrpc":"3.0","method":"health.get","id":"my-id"}"#;
        let resp = process_line(line, &ctx).unwrap();
        assert_eq!(resp.id, serde_json::json!("my-id"));
        assert_eq!(resp.error.unwrap().code, JsonRpcError::INVALID_REQUEST);
    }
}
