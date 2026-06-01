//! JSON-RPC 2.0 envelope + parsing.
//!
//! The wire is the `@io` boundary. This module owns the on-the-wire
//! shape; nothing here knows about MCP semantics — it stays close to
//! `https://www.jsonrpc.org/specification`. MCP semantics live in
//! [`crate::registry`] and [`crate::mcp`].

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{JsonRpcVersion, MethodName, RequestId};

/// JSON-RPC 2.0 request envelope.
///
/// Per JSON-RPC 2.0 §4: a request MUST carry `jsonrpc`, MAY carry
/// `params` (omitted when there are none), and MUST carry `method`.
/// `id` is REQUIRED for requests (vs. notifications, which omit it);
/// T1 supports requests only — notification handling lands in T2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: JsonRpcVersion,
    pub id: RequestId,
    pub method: MethodName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Request {
    /// Parse a JSON-RPC request from a wire-format string.
    ///
    /// Errors when the envelope is malformed (missing `jsonrpc`,
    /// wrong protocol version, missing `id`/`method`, invalid JSON).
    pub fn parse(payload: &str) -> Result<Self, ParseError> {
        serde_json::from_str(payload).map_err(ParseError::from_serde)
    }
}

/// JSON-RPC 2.0 response envelope.
///
/// Exactly one of `result` or `error` is present. The newtype here
/// is the union: callers that want stronger typing on the `result`
/// shape per-method add it on top.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: JsonRpcVersion,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

impl Response {
    /// Build a success response.
    pub fn ok(id: RequestId, result: Value) -> Self {
        Response {
            jsonrpc: JsonRpcVersion,
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Build an error response.
    pub fn err(id: RequestId, error: ResponseError) -> Self {
        Response {
            jsonrpc: JsonRpcVersion,
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 error object (§5.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ResponseError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        ResponseError {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(code: i64, message: impl Into<String>, data: Value) -> Self {
        ResponseError {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Parse-time errors. Distinct from JSON-RPC `error` responses; a
/// parse error happens when the bytes on the wire don't form a
/// well-shaped envelope at all.
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub(crate) fn from_serde(err: serde_json::Error) -> Self {
        ParseError {
            message: err.to_string(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC parse error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

// JSON-RPC 2.0 standard error codes (§5.1).
//
// These are the codes the JSON-RPC layer itself emits. MCP-spec
// codes and substrate-specific codes (e.g.
// [`crate::ERROR_NOT_IMPLEMENTED_YET`]) sit OUTSIDE the reserved
// `-32768..-32000` range, per the JSON-RPC contract.

/// JSON-RPC §5.1 — Parse error (-32700).
pub const ERROR_PARSE: i64 = -32700;
/// JSON-RPC §5.1 — Invalid Request (-32600).
pub const ERROR_INVALID_REQUEST: i64 = -32600;
/// JSON-RPC §5.1 — Method not found (-32601).
pub const ERROR_METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC §5.1 — Invalid params (-32602).
pub const ERROR_INVALID_PARAMS: i64 = -32602;
/// JSON-RPC §5.1 — Internal error (-32603).
pub const ERROR_INTERNAL: i64 = -32603;
