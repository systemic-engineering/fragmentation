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

// ---------------------------------------------------------------------------
// T3 content-tool newtypes.
//
// Per `[[feedback-no-bare-types]]`: no bare `String` / `Vec<u8>` /
// `PathBuf` crosses the content-tool surface. Each carries its own
// altitude on the wire.
// ---------------------------------------------------------------------------

/// Git-compatible content OID — hex-encoded SHA-1, 40 chars.
///
/// Returned by `fragmentation.commit`, consumed by `fragmentation.read`.
/// Internally `fragmentation::fragment::content_oid` produces these as
/// `String`s; the newtype carries the discipline at the wire boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OidString(String);

impl OidString {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for OidString {
    fn from(s: String) -> Self {
        OidString(s)
    }
}

impl From<&str> for OidString {
    fn from(s: &str) -> Self {
        OidString(s.to_string())
    }
}

/// Commit message — the human-readable text describing a `fragmentation.commit`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommitMessage(String);

impl CommitMessage {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for CommitMessage {
    fn from(s: String) -> Self {
        CommitMessage(s)
    }
}

impl From<&str> for CommitMessage {
    fn from(s: &str) -> Self {
        CommitMessage(s.to_string())
    }
}

/// In-repo path — the logical path a commit's content lives at
/// (e.g. `"src/lib.rs"`).
///
/// `String` rather than `PathBuf` at the wire altitude: the MCP
/// boundary is always UTF-8, never an OS-specific byte sequence,
/// and the substrate paths are virtual (the shard's content
/// store, not the host filesystem). The newtype is the discipline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentPath(String);

impl ContentPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for ContentPath {
    fn from(s: String) -> Self {
        ContentPath(s)
    }
}

impl From<&str> for ContentPath {
    fn from(s: &str) -> Self {
        ContentPath(s.to_string())
    }
}

/// Content payload — the bytes of a commit.
///
/// T3 wires UTF-8 string content only (the spec's `content` field is
/// a string at the JSON-RPC surface; binary content lands when the
/// MCP spec's `"contents":[{ "type":"blob", ... }]` shape is wired).
/// The newtype here lets the GREEN code be unambiguous about what
/// crosses the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommitContent(String);

impl CommitContent {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for CommitContent {
    fn from(s: String) -> Self {
        CommitContent(s)
    }
}

impl From<&str> for CommitContent {
    fn from(s: &str) -> Self {
        CommitContent(s.to_string())
    }
}

/// MCP session initialization state, per the 2024-11-05+ lifecycle.
///
/// `false` until the client sends `notifications/initialized` after
/// the `initialize` response. T5 wires the boolean flip; future
/// ticks may use this to gate methods that the spec restricts to
/// post-initialization.
///
/// Newtype rather than bare `bool` per `[[feedback-no-bare-types]]`:
/// the state is wire-altitude and load-bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionInitialized(pub bool);

impl SessionInitialized {
    pub fn as_bool(self) -> bool {
        self.0
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
