//! HTTP transport adapter for simple request/response RPC.
//!
//! This is an optional fallback transport for clients that can't use stdio or WebSocket.
//! It only supports request/response - no push notifications.

use crate::rpc::{self, RpcContext};
use crate::transport::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, trace, warn};

/// Configuration for the HTTP server.
#[derive(Clone, Debug)]
pub struct HttpConfig {
    /// Port to listen on.
    pub port: u16,
    /// Admin token for privileged operations.
    pub admin_token: Option<String>,
    /// Regular tokens for non-admin access.
    pub tokens: Vec<String>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: 3848,
            admin_token: None,
            tokens: Vec::new(),
        }
    }
}

/// HTTP server for JSON-RPC.
pub struct HttpServer {
    config: HttpConfig,
}

impl HttpServer {
    pub fn new(config: HttpConfig) -> Self {
        Self { config }
    }

    /// Run the HTTP server.
    pub async fn run(self, ctx: Arc<RpcContext>) {
        let addr = format!("127.0.0.1:{}", self.config.port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                error!(error = %e, addr = %addr, "failed to bind HTTP server");
                return;
            }
        };

        info!(addr = %addr, "HTTP server listening");

        let server = Arc::new(self);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let server = server.clone();
                    let ctx = ctx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, addr, ctx).await {
                            debug!(addr = %addr, error = %e, "connection error");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "failed to accept connection");
                }
            }
        }
    }

    async fn handle_connection(
        &self,
        mut stream: TcpStream,
        addr: SocketAddr,
        ctx: Arc<RpcContext>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(addr = %addr, "new HTTP connection");

        // Read HTTP request (simple parsing - we only support POST /rpc)
        let mut buffer = vec![0u8; 65536];
        let n = stream.read(&mut buffer).await?;
        let request_str = String::from_utf8_lossy(&buffer[..n]);

        trace!(addr = %addr, request = %request_str, "received HTTP request");

        // Parse HTTP headers and body
        let (headers, body) = match parse_http_request(&request_str) {
            Some(parsed) => parsed,
            None => {
                let response = http_response(400, "Bad Request", "Invalid HTTP request");
                stream.write_all(response.as_bytes()).await?;
                return Ok(());
            }
        };

        // Check method and path
        if !headers.starts_with("POST /rpc") && !headers.starts_with("POST / ") {
            let response = http_response(404, "Not Found", "Only POST /rpc is supported");
            stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }

        // Extract auth token from headers
        let is_admin = self.extract_auth(headers);

        // Create context with auth status
        let mut client_ctx = (*ctx).clone();
        client_ctx.is_admin = is_admin.unwrap_or(false);

        // Process JSON-RPC request
        let response = self.process_request(body, &client_ctx);
        let response_json = serde_json::to_string(&response)?;

        // Send HTTP response
        let http_response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            response_json.len(),
            response_json
        );

        stream.write_all(http_response.as_bytes()).await?;
        debug!(addr = %addr, "HTTP response sent");

        Ok(())
    }

    fn process_request(&self, body: &str, ctx: &RpcContext) -> JsonRpcResponse {
        // Parse JSON
        let request: JsonRpcRequest = match serde_json::from_str(body) {
            Ok(req) => req,
            Err(e) => {
                warn!(error = %e, "failed to parse JSON");
                return JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcError::parse_error(format!("Invalid JSON: {e}")),
                );
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            warn!(version = %request.jsonrpc, "invalid JSON-RPC version");
            return JsonRpcResponse::error(
                request.id.unwrap_or(Value::Null),
                JsonRpcError::invalid_request("Expected jsonrpc: \"2.0\""),
            );
        }

        let id = request.id.unwrap_or(Value::Null);

        // Handle the request
        debug!(method = %request.method, "handling request");
        let result = rpc::handle(&request.method, request.params, ctx);

        match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::from_rpc_error(&e)),
        }
    }

    fn extract_auth(&self, headers: &str) -> Option<bool> {
        // Look for Authorization: Bearer <token>
        for line in headers.lines() {
            if line.to_lowercase().starts_with("authorization:") {
                let value = line.splitn(2, ':').nth(1)?.trim();
                if value.to_lowercase().starts_with("bearer ") {
                    let token = value[7..].trim();
                    return self.authenticate(token);
                }
            }
        }
        // No auth header - allow unauthenticated access if no tokens configured
        if self.config.admin_token.is_none() && self.config.tokens.is_empty() {
            Some(false)
        } else {
            None
        }
    }

    fn authenticate(&self, token: &str) -> Option<bool> {
        if let Some(admin_token) = &self.config.admin_token {
            if token == admin_token {
                return Some(true);
            }
        }
        if self.config.tokens.contains(&token.to_string()) {
            return Some(false);
        }
        None
    }
}

/// Parse a simple HTTP request into headers and body.
fn parse_http_request(request: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = request.splitn(2, "\r\n\r\n").collect();
    if parts.len() == 2 {
        Some((parts[0], parts[1]))
    } else {
        // Try with just \n\n
        let parts: Vec<&str> = request.splitn(2, "\n\n").collect();
        if parts.len() == 2 {
            Some((parts[0], parts[1]))
        } else {
            None
        }
    }
}

/// Create a simple HTTP response.
fn http_response(status: u16, status_text: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: text/plain\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        status,
        status_text,
        body.len(),
        body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = HttpConfig::default();
        assert_eq!(config.port, 3848);
        assert!(config.admin_token.is_none());
        assert!(config.tokens.is_empty());
    }

    #[test]
    fn parse_valid_request() {
        let request = "POST /rpc HTTP/1.1\r\nHost: localhost\r\n\r\n{\"jsonrpc\":\"2.0\"}";
        let (headers, body) = parse_http_request(request).unwrap();
        assert!(headers.contains("POST /rpc"));
        assert!(body.contains("jsonrpc"));
    }

    #[test]
    fn parse_request_with_lf_only() {
        let request = "POST /rpc HTTP/1.1\nHost: localhost\n\n{\"jsonrpc\":\"2.0\"}";
        let (headers, body) = parse_http_request(request).unwrap();
        assert!(headers.contains("POST /rpc"));
        assert!(body.contains("jsonrpc"));
    }

    #[test]
    fn authenticate_tokens() {
        let config = HttpConfig {
            port: 3848,
            admin_token: Some("admin123".to_string()),
            tokens: vec!["user456".to_string()],
        };
        let server = HttpServer::new(config);

        assert_eq!(server.authenticate("admin123"), Some(true));
        assert_eq!(server.authenticate("user456"), Some(false));
        assert_eq!(server.authenticate("invalid"), None);
    }
}
