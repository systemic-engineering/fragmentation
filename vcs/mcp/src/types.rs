//! Wire-altitude newtypes.
//!
//! Per `[[feedback-no-bare-types]]`: no bare primitives cross the
//! MCP wire. `RequestId`, `MethodName`, `ToolName` carry the
//! substrate's discipline into JSON-RPC.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request `id` field.
///
/// The MCP spec (2025-06-18) permits string or number ids; T1 ships
/// `u64` only — sufficient for stdio sessions where the client
/// numbers requests sequentially. String ids are a T2+ refinement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(pub u64);

impl RequestId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for RequestId {
    fn from(value: u64) -> Self {
        RequestId(value)
    }
}

/// JSON-RPC `method` field — e.g. `"tools/list"`, `"tools/call"`,
/// `"initialize"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MethodName(String);

impl MethodName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MethodName {
    fn from(s: &str) -> Self {
        MethodName(s.to_string())
    }
}

impl From<String> for MethodName {
    fn from(s: String) -> Self {
        MethodName(s)
    }
}

/// MCP tool name — e.g. `"fragmentation.commit"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(String);

impl ToolName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ToolName {
    fn from(s: &str) -> Self {
        ToolName(s.to_string())
    }
}

impl From<String> for ToolName {
    fn from(s: String) -> Self {
        ToolName(s)
    }
}

/// JSON-RPC 2.0 protocol version sentinel.
///
/// The literal `"2.0"`. A unit-like newtype: type-system discipline at
/// the protocol-version altitude. Any `Request` or `Response` carries
/// this; a malformed payload that omits or mis-types `jsonrpc` is
/// rejected at parse-time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JsonRpcVersion;

impl JsonRpcVersion {
    pub const LITERAL: &'static str = "2.0";

    pub fn as_str(&self) -> &'static str {
        Self::LITERAL
    }
}

impl Serialize for JsonRpcVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::LITERAL)
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == Self::LITERAL {
            Ok(JsonRpcVersion)
        } else {
            Err(serde::de::Error::custom(format!(
                "expected JSON-RPC version {}, got {s}",
                Self::LITERAL
            )))
        }
    }
}
