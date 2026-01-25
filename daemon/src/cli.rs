//! CLI commands for daemon management.
//!
//! Client commands (health, status, events, self-test) connect to a running daemon
//! via HTTP and issue RPC requests.

use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

/// Default HTTP port for client connections.
pub const DEFAULT_PORT: u16 = 9847;

/// HTTP client for daemon RPC calls.
pub struct DaemonClient {
    host: String,
    port: u16,
    admin_auth_header: Option<String>,
}

impl DaemonClient {
    pub fn new(port: u16) -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port,
            admin_auth_header: None,
        }
    }

    pub fn with_admin_token(mut self, value: String) -> Self {
        self.admin_auth_header = Some(format!("Bearer {value}"));
        self
    }

    /// Make an RPC call to the daemon.
    pub fn call(&self, method: &str, params: Value) -> Result<Value, CliError> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let body = serde_json::to_string(&request)?;

        // Build HTTP request
        let mut http_request = format!(
            "POST /rpc HTTP/1.1\r\n\
             Host: {}:{}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n",
            self.host,
            self.port,
            body.len()
        );

        if let Some(ref header) = self.admin_auth_header {
            http_request.push_str(&format!("Authorization: {header}\r\n"));
        }

        http_request.push_str("\r\n");
        http_request.push_str(&body);

        // Connect and send
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr).map_err(|e| {
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                CliError::DaemonNotRunning
            } else {
                CliError::Connection(e.to_string())
            }
        })?;

        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .ok();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .ok();

        stream
            .write_all(http_request.as_bytes())
            .map_err(|e| CliError::Connection(e.to_string()))?;

        // Read response
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|e| CliError::Connection(e.to_string()))?;

        let response_str = String::from_utf8_lossy(&response);

        // Parse HTTP response (find body after \r\n\r\n)
        let body_start = response_str
            .find("\r\n\r\n")
            .ok_or_else(|| CliError::Protocol("Invalid HTTP response".to_string()))?;
        let body = &response_str[body_start + 4..];

        // Parse JSON-RPC response
        let rpc_response: RpcResponse =
            serde_json::from_str(body).map_err(|e| CliError::Protocol(e.to_string()))?;

        if let Some(error) = rpc_response.error {
            Err(CliError::Rpc {
                code: error.code,
                message: error.message,
            })
        } else {
            Ok(rpc_response.result.unwrap_or(Value::Null))
        }
    }
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Value,
    result: Option<Value>,
    error: Option<RpcErrorData>,
}

#[derive(Debug, Deserialize)]
struct RpcErrorData {
    code: String,
    message: String,
}

/// CLI errors.
#[derive(Debug)]
pub enum CliError {
    DaemonNotRunning,
    Connection(String),
    Protocol(String),
    Rpc { code: String, message: String },
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DaemonNotRunning => {
                write!(f, "Daemon is not running. Start with: ntm-tracker-daemon start")
            }
            Self::Connection(msg) => write!(f, "Connection error: {msg}"),
            Self::Protocol(msg) => write!(f, "Protocol error: {msg}"),
            Self::Rpc { code, message } => write!(f, "RPC error [{code}]: {message}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

/// Format and print a value according to the output format.
pub fn print_output(value: &Value, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
        }
        OutputFormat::Text => {
            print_text_output(value, 0);
        }
    }
}

fn print_text_output(value: &Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    match value {
        Value::Null => println!("{prefix}null"),
        Value::Bool(b) => println!("{prefix}{b}"),
        Value::Number(n) => println!("{prefix}{n}"),
        Value::String(s) => println!("{prefix}{s}"),
        Value::Array(arr) => {
            for item in arr {
                print_text_output(item, indent);
            }
        }
        Value::Object(obj) => {
            for (key, val) in obj {
                match val {
                    Value::Object(_) | Value::Array(_) => {
                        println!("{prefix}{key}:");
                        print_text_output(val, indent + 1);
                    }
                    _ => {
                        print!("{prefix}{key}: ");
                        print_simple_value(val);
                    }
                }
            }
        }
    }
}

fn print_simple_value(value: &Value) {
    match value {
        Value::Null => println!("null"),
        Value::Bool(b) => println!("{b}"),
        Value::Number(n) => println!("{n}"),
        Value::String(s) => println!("{s}"),
        _ => println!("{value}"),
    }
}

/// Execute the 'health' command.
pub fn cmd_health(port: u16, format: OutputFormat, admin_token: Option<String>) -> Result<(), CliError> {
    let mut client = DaemonClient::new(port);
    if let Some(value) = admin_token {
        client = client.with_admin_token(value);
    }

    let result = client.call("health.get", json!({}))?;
    print_output(&result, format);
    Ok(())
}

/// Execute the 'status' command (list sessions).
pub fn cmd_status(port: u16, format: OutputFormat, admin_token: Option<String>) -> Result<(), CliError> {
    let mut client = DaemonClient::new(port);
    if let Some(value) = admin_token {
        client = client.with_admin_token(value);
    }

    let result = client.call("sessions.list", json!({}))?;

    if format == OutputFormat::Text {
        // Pretty print session summary
        if let Some(sessions) = result.as_array() {
            if sessions.is_empty() {
                println!("No active sessions");
            } else {
                println!("Sessions ({}):", sessions.len());
                for session in sessions {
                    let name = session.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let status = session.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                    let pane_count = session.get("paneCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  {name}: {status} ({pane_count} panes)");
                }
            }
        } else {
            print_output(&result, format);
        }
    } else {
        print_output(&result, format);
    }
    Ok(())
}

/// Execute the 'events' command.
pub fn cmd_events(
    port: u16,
    format: OutputFormat,
    admin_token: Option<String>,
    limit: Option<u32>,
) -> Result<(), CliError> {
    let mut client = DaemonClient::new(port);
    if let Some(value) = admin_token {
        client = client.with_admin_token(value);
    }

    let params = json!({
        "limit": limit.unwrap_or(20),
    });
    let result = client.call("events.list", params)?;

    if format == OutputFormat::Text {
        if let Some(events) = result.as_array() {
            if events.is_empty() {
                println!("No recent events");
            } else {
                println!("Recent events ({}):", events.len());
                for event in events {
                    let event_type = event.get("eventType").and_then(|v| v.as_str()).unwrap_or("?");
                    let session = event.get("sessionUid").and_then(|v| v.as_str()).unwrap_or("?");
                    let detected_at = event.get("detectedAt").and_then(|v| v.as_i64()).unwrap_or(0);
                    let severity = event.get("severity").and_then(|v| v.as_str());

                    let sev_str = severity.map(|s| format!(" [{s}]")).unwrap_or_default();
                    println!("  [{detected_at}] {event_type}{sev_str} - session: {session}");
                }
            }
        } else {
            print_output(&result, format);
        }
    } else {
        print_output(&result, format);
    }
    Ok(())
}

/// Execute the 'self-test' command.
pub fn cmd_self_test(port: u16, format: OutputFormat, admin_token: Option<String>) -> Result<(), CliError> {
    let mut client = DaemonClient::new(port);
    if let Some(value) = admin_token {
        client = client.with_admin_token(value);
    }

    let result = client.call("debug.selfTest", json!({}))?;

    if format == OutputFormat::Text {
        let ok = result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let checks = result.get("checks").and_then(|v| v.as_array());

        if ok {
            println!("✓ All checks passed");
        } else {
            println!("✗ Some checks failed");
        }

        if let Some(checks) = checks {
            for check in checks {
                let name = check.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let check_ok = check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                let detail = check.get("detail").and_then(|v| v.as_str());

                let status = if check_ok { "✓" } else { "✗" };
                let detail_str = detail.map(|d| format!(": {d}")).unwrap_or_default();
                println!("  {status} {name}{detail_str}");
            }
        }
    } else {
        print_output(&result, format);
    }
    Ok(())
}

/// Execute the 'config' command.
pub fn cmd_config(port: u16, format: OutputFormat, admin_token: Option<String>) -> Result<(), CliError> {
    let mut client = DaemonClient::new(port);
    if let Some(value) = admin_token {
        client = client.with_admin_token(value);
    }

    let result = client.call("config.get", json!({}))?;
    print_output(&result, format);
    Ok(())
}

/// Stop a running daemon by sending a shutdown signal.
pub fn cmd_stop(pid_file: Option<PathBuf>) -> Result<(), CliError> {
    let pid_path = pid_file.unwrap_or_else(|| {
        crate::service::data_dir().join("daemon.pid")
    });

    if !pid_path.exists() {
        return Err(CliError::DaemonNotRunning);
    }

    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .map_err(|_| CliError::Protocol("Invalid PID in file".to_string()))?;

    #[cfg(unix)]
    {
        // Send SIGTERM
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        if result == 0 {
            println!("Sent shutdown signal to daemon (PID {pid})");
            Ok(())
        } else {
            Err(CliError::DaemonNotRunning)
        }
    }

    #[cfg(not(unix))]
    {
        Err(CliError::Protocol("Stop command not supported on this platform".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_text_handles_simple_values() {
        let value = json!({"key": "value", "number": 42});
        // Should not panic
        print_output(&value, OutputFormat::Text);
    }

    #[test]
    fn output_format_json_produces_valid_json() {
        let value = json!({"key": "value"});
        // Should not panic
        print_output(&value, OutputFormat::Json);
    }

    #[test]
    fn daemon_client_creation() {
        let client = DaemonClient::new(9847);
        assert_eq!(client.port, 9847);
        assert!(client.admin_auth_header.is_none());

        let client = client.with_admin_token("secret".to_string());
        assert_eq!(client.admin_auth_header, Some("Bearer secret".to_string()));
    }
}
