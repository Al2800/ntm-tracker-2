//! WebSocket transport adapter with token authentication.
//!
//! This is an optional transport for clients that prefer WebSocket over stdio.
//! It supports full duplex communication with push notifications.

use crate::rpc::{self, RpcContext};
use crate::transport::{JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, trace, warn};

/// Configuration for the WebSocket server.
#[derive(Clone, Debug)]
pub struct WsConfig {
    /// Port to listen on.
    pub port: u16,
    /// Admin token for privileged operations.
    pub admin_token: Option<String>,
    /// Regular tokens for non-admin access.
    pub tokens: Vec<String>,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            port: 3847,
            admin_token: None,
            tokens: Vec::new(),
        }
    }
}

/// Connected client state.
#[allow(dead_code)]
struct Client {
    addr: SocketAddr,
    is_admin: bool,
    subscriptions: Vec<String>,
}

/// WebSocket server state.
pub struct WsServer {
    config: WsConfig,
    clients: Arc<RwLock<HashMap<SocketAddr, Client>>>,
    notification_tx: broadcast::Sender<JsonRpcNotification>,
}

impl WsServer {
    pub fn new(config: WsConfig) -> Self {
        let (notification_tx, _) = broadcast::channel(256);
        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
            notification_tx,
        }
    }

    /// Get a sender for broadcasting notifications to all connected clients.
    pub fn notification_sender(&self) -> broadcast::Sender<JsonRpcNotification> {
        self.notification_tx.clone()
    }

    /// Run the WebSocket server.
    pub async fn run(self, ctx: Arc<RpcContext>) {
        let addr = format!("127.0.0.1:{}", self.config.port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                error!(error = %e, addr = %addr, "failed to bind WebSocket server");
                return;
            }
        };

        info!(addr = %addr, "WebSocket server listening");

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
        stream: TcpStream,
        addr: SocketAddr,
        ctx: Arc<RpcContext>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(addr = %addr, "new WebSocket connection");

        // Extract auth info during WebSocket handshake
        let auth_result = Arc::new(std::sync::Mutex::new(None::<bool>));
        let auth_result_clone = auth_result.clone();
        let config_clone = self.config.clone();

        let callback = move |req: &Request, response: Response| -> Result<Response, ErrorResponse> {
            // Try to extract token from query string first
            let uri = req.uri();
            let mut token: Option<&str> = None;

            // Parse query string for ?token=xxx
            if let Some(query) = uri.query() {
                for pair in query.split('&') {
                    if let Some(value) = pair.strip_prefix("token=") {
                        token = Some(value);
                        break;
                    }
                }
            }

            // If no query token, try Authorization header
            if token.is_none() {
                if let Some(auth_header) = req.headers().get("authorization") {
                    if let Ok(auth_str) = auth_header.to_str() {
                        if auth_str.to_lowercase().starts_with("bearer ") {
                            token = Some(&auth_str[7..]);
                        }
                    }
                }
            }

            // Authenticate the token
            let is_admin = if let Some(tok) = token {
                // Check admin token
                if let Some(admin_token) = &config_clone.admin_token {
                    if tok == admin_token {
                        Some(true)
                    } else if config_clone.tokens.contains(&tok.to_string()) {
                        Some(false)
                    } else {
                        None // Invalid token
                    }
                } else if config_clone.tokens.contains(&tok.to_string()) {
                    Some(false)
                } else {
                    None // Invalid token
                }
            } else {
                // No token provided - allow only if no tokens are configured
                if config_clone.admin_token.is_none() && config_clone.tokens.is_empty() {
                    Some(false) // No auth required, non-admin access
                } else {
                    None // Auth required but not provided
                }
            };

            // Store auth result for use after handshake
            if let Ok(mut guard) = auth_result_clone.lock() {
                *guard = is_admin;
            }

            // Reject connection during handshake if auth failed
            if is_admin.is_none() {
                let reject = tokio_tungstenite::tungstenite::http::Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Some("Unauthorized: missing or invalid token".to_string()))
                    .unwrap();
                return Err(reject);
            }

            Ok(response)
        };

        // Perform WebSocket handshake with auth callback
        let ws_stream = match tokio_tungstenite::accept_hdr_async(stream, callback).await {
            Ok(ws) => ws,
            Err(e) => {
                // This includes auth rejections - log and return
                warn!(addr = %addr, error = %e, "WebSocket handshake failed");
                return Ok(());
            }
        };

        // Get auth result (should always be Some at this point since we reject None in callback)
        let is_admin = match auth_result.lock() {
            Ok(guard) => guard.unwrap_or(false),
            Err(_) => false,
        };

        let (mut write, mut read) = ws_stream.split();

        // Register client
        {
            let mut clients = self.clients.write().await;
            clients.insert(
                addr,
                Client {
                    addr,
                    is_admin,
                    subscriptions: Vec::new(),
                },
            );
        }

        info!(addr = %addr, is_admin = %is_admin, "WebSocket client connected");

        // Subscribe to notifications
        let mut notification_rx = self.notification_tx.subscribe();

        // Create a channel for outgoing messages
        let (tx, mut rx) = mpsc::channel::<String>(32);

        // Spawn a task to forward messages to the WebSocket
        let write_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        if let Err(e) = write.send(Message::Text(msg.into())).await {
                            debug!(error = %e, "failed to send message");
                            break;
                        }
                    }
                    else => break,
                }
            }
        });

        // Create a context with admin status
        let mut client_ctx = (*ctx).clone();
        client_ctx.is_admin = is_admin;

        // Send hello notification immediately after connect for version/capability handshake.
        let hello = JsonRpcNotification::new("core.hello", rpc::hello_payload(&client_ctx));
        if let Ok(json) = serde_json::to_string(&hello) {
            let _ = tx.send(json).await;
        }

        // Process incoming messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let text_str = text.to_string();
                    trace!(addr = %addr, msg = %text_str, "received message");
                    if let Some(response) = self.process_message(&text_str, &client_ctx) {
                        let json = serde_json::to_string(&response)?;
                        if tx.send(json).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    debug!(addr = %addr, "client sent close");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    // Pong is handled automatically by tungstenite
                }
                Ok(_) => {
                    // Ignore binary and other message types
                }
                Err(e) => {
                    debug!(addr = %addr, error = %e, "read error");
                    break;
                }
            }

            // Also check for notifications to forward
            while let Ok(notification) = notification_rx.try_recv() {
                let json = serde_json::to_string(&notification)?;
                if tx.send(json).await.is_err() {
                    break;
                }
            }
        }

        // Cleanup
        drop(tx);
        write_task.abort();

        {
            let mut clients = self.clients.write().await;
            clients.remove(&addr);
        }

        info!(addr = %addr, "WebSocket client disconnected");
        Ok(())
    }

    fn process_message(&self, text: &str, ctx: &RpcContext) -> Option<JsonRpcResponse> {
        // Parse JSON
        let request: JsonRpcRequest = match serde_json::from_str(text) {
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

    /// Authenticate a token and return whether it's an admin token.
    /// Returns `Some(true)` for admin tokens, `Some(false)` for regular tokens,
    /// and `None` for invalid tokens.
    pub fn authenticate(&self, token: &str) -> Option<bool> {
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

/// Create a notification sender for pushing events to WebSocket clients.
pub fn notification_channel() -> (broadcast::Sender<JsonRpcNotification>, broadcast::Receiver<JsonRpcNotification>) {
    broadcast::channel(256)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = WsConfig::default();
        assert_eq!(config.port, 3847);
        assert!(config.admin_token.is_none());
        assert!(config.tokens.is_empty());
    }

    #[test]
    fn authenticate_admin() {
        let config = WsConfig {
            port: 3847,
            admin_token: Some("admin123".to_string()),
            tokens: vec!["user456".to_string()],
        };
        let server = WsServer::new(config);

        assert_eq!(server.authenticate("admin123"), Some(true));
        assert_eq!(server.authenticate("user456"), Some(false));
        assert_eq!(server.authenticate("invalid"), None);
    }

    #[test]
    fn authenticate_regular_token_without_admin() {
        let config = WsConfig {
            port: 3847,
            admin_token: None,
            tokens: vec!["user123".to_string(), "user456".to_string()],
        };
        let server = WsServer::new(config);

        assert_eq!(server.authenticate("user123"), Some(false));
        assert_eq!(server.authenticate("user456"), Some(false));
        assert_eq!(server.authenticate("invalid"), None);
    }

    #[test]
    fn authenticate_empty_config_rejects_all() {
        let server = WsServer::new(WsConfig::default());
        // With no tokens configured, authenticate() returns None for any token
        // (but handle_connection allows unauthenticated access in this case)
        assert_eq!(server.authenticate("anything"), None);
    }

    #[test]
    fn authenticate_admin_takes_priority() {
        // If same token is in both admin_token and tokens, admin wins
        let config = WsConfig {
            port: 3847,
            admin_token: Some("shared".to_string()),
            tokens: vec!["shared".to_string()],
        };
        let server = WsServer::new(config);

        assert_eq!(server.authenticate("shared"), Some(true)); // Admin takes priority
    }
}
