//! Transport adapters for JSON-RPC communication.
//!
//! All transports use the same RPC handlers - they just differ in how
//! they receive requests and send responses/notifications.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod http;
pub mod stdio;
pub mod ws;

/// JSON-RPC 2.0 request structure.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    /// If present, this is a request expecting a response.
    /// If absent, this is a notification (no response expected).
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 response structure.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC 2.0 error structure.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Standard JSON-RPC error codes
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Application error codes (reserved range: -32000 to -32099)
    pub const UNAUTHORIZED: i32 = -32001;
    pub const FORBIDDEN: i32 = -32002;
    pub const RATE_LIMITED: i32 = -32003;
    pub const NOT_FOUND: i32 = -32004;
    pub const STALE_CURSOR: i32 = -32005;
    pub const UNSUPPORTED: i32 = -32006;
    pub const DEGRADED: i32 = -32007;

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: message.into(),
            data: None,
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: message.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: format!("Method not found: {method}"),
            data: None,
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR,
            message: message.into(),
            data: None,
        }
    }

    /// Convert from application RpcError to JSON-RPC error
    pub fn from_rpc_error(err: &crate::rpc::RpcError) -> Self {
        let code = match err.code {
            crate::rpc::CODE_UNAUTHORIZED => Self::UNAUTHORIZED,
            crate::rpc::CODE_FORBIDDEN => Self::FORBIDDEN,
            crate::rpc::CODE_RATE_LIMITED => Self::RATE_LIMITED,
            crate::rpc::CODE_NOT_FOUND => Self::NOT_FOUND,
            crate::rpc::CODE_STALE_CURSOR => Self::STALE_CURSOR,
            crate::rpc::CODE_UNSUPPORTED => Self::UNSUPPORTED,
            crate::rpc::CODE_DEGRADED => Self::DEGRADED,
            crate::rpc::CODE_INVALID_PARAMS => Self::INVALID_PARAMS,
            _ => Self::INTERNAL_ERROR,
        };

        Self {
            code,
            message: err.message.clone(),
            data: err.data.clone(),
        }
    }
}

/// JSON-RPC 2.0 notification (server -> client push).
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: &'static str,
    pub method: String,
    pub params: Value,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            method: method.into(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_request_with_id() {
        let json = r#"{"jsonrpc":"2.0","method":"health.get","params":{},"id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "health.get");
        assert_eq!(req.id, Some(Value::Number(1.into())));
    }

    #[test]
    fn deserialize_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"compact"}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "event");
        assert!(req.id.is_none());
    }

    #[test]
    fn serialize_success_response() {
        let resp = JsonRpcResponse::success(Value::Number(1.into()), Value::Bool(true));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\":true"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn serialize_error_response() {
        let resp = JsonRpcResponse::error(
            Value::Number(1.into()),
            JsonRpcError::method_not_found("unknown"),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("result"));
    }
}
