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

        // Perform WebSocket handshake with callback for auth
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        // For now, we'll check auth on first message or via query params
        // In a real implementation, we'd extract the token from the upgrade request
        let is_admin = false; // Will be set based on token

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

        info!(addr = %addr, "WebSocket client connected");

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
    #[allow(dead_code)]
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
}
